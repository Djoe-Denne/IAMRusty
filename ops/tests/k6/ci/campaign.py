"""Hosted Ubuntu parent for the unchanged health/OAuth/SAN stimuli, no Cargo."""
import argparse
import base64
from copy import deepcopy
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.parse
import urllib.error
import urllib.request
import ssl
import uuid

import yaml

from artifacts import premerge_job
from binaries import RECOVERY_ROLE, ROOT, Refused, command, require, sha, strict_json
import browser as browser_profile
import campaign_browser
import evidence
import inputs
import late_check
import pki
from recovery_monolith import create_monolith, create_standalone
from runtime import Runtime, NS, hosted, write_json
from session_cutoff import DIRECT_PORT, HostedRecovery
from session_replica import SessionReplica

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import Child, Rejected, child_environment, new_output, private_path

HERE = Path(__file__).resolve().parent
ERROR_COUNTERS = ("upstream_cx_connect_fail", "upstream_cx_connect_timeout", "ssl.fail_verify_san", "ssl.fail_verify_error")
STAT_FILTER = r"^cluster\.(iam|ext_authz)\.(upstream_rq_total|upstream_rq_5xx|upstream_cx_connect_fail|upstream_cx_connect_timeout|ssl\.(handshake|fail_verify_san|fail_verify_error))$"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        return None


def iam_endpoint_ready(service, slices, pod, clusters):
    """Envoy resolves a ClusterIP Service; EndpointSlices bind it to the Pod."""
    routed = any(cluster.get("name") == "iam" and any(
        host.get("address", {}).get("socket_address", {}).get("address") == service["spec"]["clusterIP"]
        for host in cluster.get("host_statuses", [])) for cluster in clusters)
    selected = any(
        any(owner.get("kind") == "Service" and owner.get("uid") == service["metadata"]["uid"]
            for owner in item["metadata"].get("ownerReferences", []))
        and any(port.get("port") == 8443 for port in item.get("ports", []))
        and any(endpoint.get("conditions", {}).get("ready") is True
                and endpoint.get("targetRef", {}).get("uid") == pod["metadata"]["uid"]
                and pod["status"]["podIP"] in endpoint.get("addresses", [])
                for endpoint in item.get("endpoints", []))
        for item in slices)
    return routed and selected


def stimulus_environment(output, root, state, is_browser):
    env = child_environment(output)
    env.update(K6_RUN_ID=state["run_id"], K6_ORIGIN="https://127.0.0.1:18443",
               K6_CAMPAIGN_FILE=str(root / "private/target.json"),
               K6_SUMMARY_PATH=str(output / "private/summary.json"),
               SSL_CERT_FILE=str(root / "private/pki/ca-bundle.crt"))
    if is_browser:
        env.update(K6_BROWSER_EXECUTABLE_PATH=state["tools"]["chromium"]["path"],
                   K6_EXPECTED_BROWSER_VERSION=state["tools"]["chromium"]["version"],
                   K6_BROWSER_ARGS="enable-logging,log-level=2,log-file=" + str(output / "private/chromium.log"))
    return env


def read_failed_summary(path, run_id, case):
    try:
        if path.is_symlink() or path.parent.is_symlink():
            return {"summary_status": "invalid"}
        if not path.exists():
            return {"summary_status": "missing"}
        if not path.is_file():
            return {"summary_status": "invalid"}
        with path.open("rb") as stream:
            value = evidence.strict_json(stream.read(evidence.SUMMARY_LIMIT + 1))
        value = evidence.inspect_failed_summary(value, run_id, case)
        return {"summary_status": "captured", "failed_summary": value}
    except (OSError, evidence.EvidenceError):
        return {"summary_status": "invalid"}


class Campaign:
    def __init__(self, runtime):
        self.r = runtime
        self.root = runtime.directory
        self.state = runtime.state
        self.public = self.root / "public"

    def publish(self, name, value):
        require(re.fullmatch(r"[a-z0-9-]+\.json", name), "E2E_PUBLIC_FILE_NAME")
        write_json(self.public / name, value)

    def publish_case(self, case, value):
        """Only receiver-validated values may enter the upload directory."""
        value = evidence.accept_summary(json.dumps(value).encode(), self.state["run_id"], case, 0)
        directory = self.public / "cases"
        require(not directory.is_symlink(), "E2E_PUBLIC_CASE_DIRECTORY")
        directory.mkdir(mode=0o700, exist_ok=True)
        target = directory / (case + ".json")
        require(not target.exists() and not target.is_symlink(), "E2E_PUBLIC_CASE_COLLISION")
        write_json(target, value)

    def create(self, value):
        require(value["kind"] in {"Namespace", "Secret", "ConfigMap", "Deployment", "Service",
                                  "PersistentVolumeClaim", "NetworkPolicy", "Job"}, "E2E_RESOURCE_KIND")
        value = json.loads(json.dumps(value))
        metadata = value.setdefault("metadata", {})
        require("generateName" not in metadata and re.fullmatch(r"[a-z0-9][a-z0-9-]{0,62}", metadata["name"]), "E2E_RESOURCE_NAME")
        if value["kind"] == "Namespace":
            require(metadata["name"] == NS, "E2E_NAMESPACE")
        else:
            require(metadata.get("namespace", NS) == NS, "E2E_NAMESPACE")
            metadata["namespace"] = NS
        metadata.setdefault("labels", {})["aiforall.dev/k6-run"] = self.state["run_id"]
        path = self.root / "private/create.json"
        write_json(path, value)
        self.state["stage"] = "create:" + value["kind"] + ":" + metadata["name"]
        self.r.save()
        identity = self.r.k(["create", "--filename", str(path), "--output", "jsonpath={.metadata.uid}"]).decode().strip()
        require(re.fullmatch(r"[a-f0-9-]{36}", identity), "E2E_RESOURCE_UID")
        receipt = {"kind": value["kind"], "name": metadata["name"], "uid": identity}
        self.state["resources"].append(receipt)
        self.r.save()
        return receipt

    def secret(self, name, data):
        return self.create({"apiVersion": "v1", "kind": "Secret", "metadata": {"name": name},
                            "type": "Opaque", "data": {key: base64.b64encode(value if isinstance(value, bytes)
                                                                            else value.encode()).decode()
                                                        for key, value in data.items()}})

    def render(self, chart, values):
        path = self.root / "private/render-values.json"
        write_json(path, values)
        raw = self.r.cli("helm", ["template", "k6-ci", str(chart), "--namespace", NS, "-f", str(path)])
        documents = list(yaml.safe_load_all(raw))
        result = []
        for item in documents:
            if item:
                result.extend(item["items"] if item.get("kind") == "List" else [item])
        require(bool(result), "E2E_EMPTY_RENDER")
        return result

    def backings(self, session_replica=None, **selection):
        values = yaml.safe_load((HERE / "values.yaml").read_text(encoding="utf-8"))
        for name, enabled in selection.items():
            group, key = name.split("_")
            require(group in {"postgres", "openfga", "localstack", "network"} and key in {"create", "initialize"}, "E2E_BACKING_SELECTION")
            values.setdefault(group, {})[key] = enabled
        for item in self.render(ROOT / "ops/deploy/charts/kind-minimal", values):
            if session_replica is not None:
                session_replica.postgres_options(item)
            self.create(item)

    def rollout(self, name):
        self.state["stage"] = "rollout:" + name
        self.r.save()
        try:
            self.r.k(["rollout", "status", "deployment/" + name, "--timeout=180s"], timeout=200)
        except Exception:
            self.diagnostics()
            raise

    def diagnostics(self):
        """Only typed status fields; never messages, env, URLs or service logs."""
        try:
            value = strict_json(self.r.k(["get", "pods", "--output", "json"]))
            rows = []
            for pod in value["items"]:
                statuses = []
                for item in pod.get("status", {}).get("initContainerStatuses", []) + pod.get("status", {}).get("containerStatuses", []):
                    state = item.get("state", {})
                    reason = next((v.get("reason", "") for k, v in state.items() if k in ("waiting", "terminated")), "")
                    statuses.append({"name": item["name"], "ready": item["ready"],
                                     "restarts": item["restartCount"], "reason": reason if re.fullmatch(r"[A-Za-z0-9_]{0,80}", reason) else "WITHHELD"})
                rows.append({"name": pod["metadata"]["name"], "phase": pod.get("status", {}).get("phase"), "containers": statuses})
            self.publish("startup.json", {"stage": self.state.get("stage"), "pods": rows})
        except Exception:
            self.publish("startup.json", {"status": "UNAVAILABLE", "stage": self.state.get("stage")})

    def pod(self, role):
        selector = "aiforall.dev/k6-run=" + self.state["run_id"] + ",aiforall.dev/k6-role=" + role
        value = strict_json(self.r.k(["get", "pods", "--selector", selector, "--output", "json"]))
        live = [pod for pod in value["items"] if not pod["metadata"].get("deletionTimestamp")]
        require(len(live) == 1 and live[0]["status"]["phase"] == "Running", "E2E_FIXTURE_POD")
        pod = live[0]
        owner = pod["metadata"]["ownerReferences"][0]
        rs = strict_json(self.r.k(["get", "replicaset", owner["name"], "--output", "json"]))
        receipt = self.receipt("Deployment", self.state["name"] + "-" + role)
        require(owner["uid"] == rs["metadata"]["uid"] and rs["metadata"]["ownerReferences"][0]["uid"] == receipt["uid"], "E2E_POD_OWNER")
        return pod

    def receipt(self, kind, name):
        values = [r for r in self.state["resources"] if r["kind"] == kind and r["name"] == name]
        require(len(values) == 1, "E2E_RESOURCE_RECEIPT")
        return values[0]

    def identity(self):
        pod = self.pod("envoy")
        statuses = pod["status"]["containerStatuses"]
        require(len(statuses) == 1 and statuses[0]["ready"] is True, "E2E_EDGE_NOT_READY")
        return (pod["metadata"]["uid"], statuses[0]["containerID"], statuses[0]["restartCount"])

    def postgres_pod(self):
        value = strict_json(self.r.k(["get", "pods", "-l", "app.kubernetes.io/name=postgres", "-o", "json"]))
        require(len(value["items"]) == 1 and value["items"][0]["status"]["phase"] == "Running", "E2E_POSTGRES_POD")
        return value["items"][0]["metadata"]["name"]

    def sql(self, query, database="postgres", *, record_stage=True):
        writer = self.state["values"]["database"]
        allowed = ("postgres", "openfga_dev", writer, writer + "_admission")
        if RECOVERY_ROLE in self.state.get("images", {}).get("roles", {}):
            allowed += (writer + "_manifesto",)
        require(database in allowed, "E2E_SQL_DATABASE")
        if record_stage:
            self.state["stage"] = "sql:" + database
            self.r.save()
        # The shared readiness profile sets PGHOSTADDR=127.0.0.1. Like the
        # native helper, remove inherited TCP selectors for this socket-only
        # administrative invocation; never relax network TLS or pg_hba.
        script = query.rstrip()
        if not script.endswith(";"):
            script += ";"
        return self.r.k(["exec", "-i", self.postgres_pod(), "-c", "postgres", "--", "env", "-u", "PGHOSTADDR", "-u", "PGHOST",
                         "psql", "--host", "/var/run/postgresql",
                         "--username", "postgres", "--dbname", database, "--no-password", "--no-psqlrc",
                         "--quiet", "--tuples-only", "--no-align", "--set", "ON_ERROR_STOP=1",
                         "--file", "-"], stdin=(script + "\n").encode("utf-8")).decode().strip()

    def forward(self, role, local, remote):
        pod = self.pod(role)
        args = [self.state["tools"]["kubectl"]["path"], "--kubeconfig", self.state["kubeconfig"],
                "--context", "kind-" + self.state["cluster"], "--namespace", NS, "port-forward",
                "--address", "127.0.0.1", "pod/" + pod["metadata"]["name"], f"{local}:{remote}"]
        stream = (self.root / "private" / f"forward-{local}.log").open("wb")
        process = subprocess.Popen(args, env=self.r.env(), stdout=stream, stderr=stream, start_new_session=True)
        stream.close()
        self.r.track_process(process, "port-forward-" + str(local))
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            require(process.poll() is None, "E2E_FORWARD_EXITED")
            try:
                with socket.create_connection(("127.0.0.1", local), timeout=0.3):
                    return
            except OSError:
                time.sleep(0.1)
        raise Refused("E2E_FORWARD_TIMEOUT")

    def http(self, url):
        allowed = ("http://127.0.0.1:19901/", "https://localhost:18445/", "https://127.0.0.1:18443/")
        require(any(url.startswith(prefix) for prefix in allowed), "E2E_HTTP_SCOPE")
        context = ssl.create_default_context(cafile=str(self.root / "private/pki/ca.crt"))
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}),
                                            urllib.request.HTTPSHandler(context=context), NoRedirect())
        try:
            with opener.open(url, timeout=10) as response:
                raw = response.read(1024 * 1024 + 1)
                require(len(raw) <= 1024 * 1024, "E2E_HTTP_LIMIT")
                return response.status, raw
        except urllib.error.HTTPError as error:
            require(error.code in (303, 403, 503), "E2E_HTTP_FAILED")
            return error.code, b""

    def snapshot(self):
        identity = list(self.identity())
        require(identity == self.state["edge_identity"], "E2E_ENVOY_REPLACED")
        status, body = self.http("http://127.0.0.1:19901/server_info")
        server = strict_json(body)
        require(status == 200 and server["state"] == "LIVE" and "/1.31.2/" in server["version"], "E2E_ENVOY_VERSION")
        epoch = server["command_line_options"]["restart_epoch"]
        if "edge_epoch" in self.state:
            require(epoch == self.state["edge_epoch"], "E2E_ENVOY_EPOCH_CHANGED")
        else:
            self.state["edge_epoch"] = epoch
            self.r.save()
        status, body = self.http("http://127.0.0.1:19901/stats?format=json&filter=" + urllib.parse.quote(STAT_FILTER, safe=""))
        value = strict_json(body)
        result = {}
        for item in value["stats"]:
            require(set(item) == {"name", "value"} and re.fullmatch(STAT_FILTER, item["name"])
                    and item["name"] not in result and evidence.integer(item["value"]), "E2E_ADMIN_COUNTER")
            result[item["name"]] = item["value"]
        return result

    def wait_endpoint(self):
        pod = self.pod("iam")
        name = self.state["name"] + "-iam"
        service = strict_json(self.r.k(["get", "service", name, "--output", "json"]))
        require(service["metadata"]["uid"] == self.receipt("Service", name)["uid"]
                and service["spec"]["clusterIP"] not in ("None", ""), "E2E_IAM_SERVICE_IDENTITY")
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            _, body = self.http("http://127.0.0.1:19901/clusters?format=json")
            clusters = strict_json(body)["cluster_statuses"]
            slices = strict_json(self.r.k(["get", "endpointslices", "--selector", "kubernetes.io/service-name=" + name,
                                            "--output", "json"]))["items"]
            if iam_endpoint_ready(service, slices, pod, clusters):
                return
            time.sleep(0.2)
        raise Refused("E2E_IAM_DNS_ACK_MISSING")

    def certificate(self, phase):
        root = self.root / "private/pki"
        role = "hive-service" if phase == "bad" else "iam-service"
        desired = {"iam-service.crt": base64.b64encode((root / (role + ".crt")).read_bytes()).decode(),
                   "iam-service.key": base64.b64encode((root / (role + ".key")).read_bytes()).decode()}
        previous = "iam-service" if phase == "bad" else "hive-service"
        original = {key: base64.b64encode((root / (previous + suffix)).read_bytes()).decode()
                    for key, suffix in (("iam-service.crt", ".crt"), ("iam-service.key", ".key"))}
        receipt = self.receipt("Secret", self.state["name"] + "-iam-tls")
        patch = [{"op": "test", "path": "/metadata/uid", "value": receipt["uid"]}]
        patch += [{"op": "test", "path": "/data/" + key, "value": value} for key, value in original.items()]
        patch += [{"op": "replace", "path": "/data/" + key, "value": value} for key, value in desired.items()]
        path = self.root / "private/certificate-patch.json"
        write_json(path, patch)
        try:
            self.r.k(["patch", "secret", receipt["name"], "--type=json", "--patch-file", str(path), "-o", "name"])
        except Refused:
            if phase == "bad":
                raise
            write_json(path, [{"op": "test", "path": "/metadata/uid", "value": receipt["uid"]}] +
                       [{"op": "test", "path": "/data/" + key, "value": value} for key, value in desired.items()])
            self.r.k(["patch", "secret", receipt["name"], "--type=json", "--patch-file", str(path), "-o", "name"])
        deployment = self.receipt("Deployment", self.state["name"] + "-iam")
        write_json(path, [{"op": "test", "path": "/metadata/uid", "value": deployment["uid"]},
                          {"op": "replace", "path": "/spec/template/metadata/annotations/aiforall.dev~1certificate-phase", "value": phase}])
        self.r.k(["patch", "deployment", deployment["name"], "--type=json", "--patch-file", str(path), "-o", "name"])
        self.rollout(deployment["name"])
        self.wait_endpoint()
        self.state["certificate_phase"] = phase
        self.r.save()

    def scenario(self, case):
        require(case in evidence.CATALOG and sha(evidence.CATALOG_PATH) == evidence.CATALOG_SHA256,
                "E2E_CATALOG_CHANGED")
        cases = self.root / "private" / "cases"
        require(not cases.is_symlink(), "E2E_CASE_DIRECTORY")
        cases.mkdir(mode=0o700, exist_ok=True)
        attempt = cases / (case + ".attempt")
        require(not attempt.exists(), "E2E_CASE_REPLAY_REFUSED")
        self.state["stage"] = "scenario:" + case
        self.r.save()
        output = new_output(str(cases / case))
        record = evidence.CATALOG[case]
        is_browser = record["lifecycle"] == "oauth"
        env = stimulus_environment(output, self.root, self.state, is_browser)
        if is_browser:
            browser_profile.trust(output / "home", self.root / "private/pki/ca.crt", "k6-" + self.state["run_id"], self.r.cli)
        if record["lifecycle"] == "san":
            env["K6_AUTH_SAN_PHASE"] = case.removeprefix("auth-envoy-san-")
        script = record["script"]
        env["K6_CASE_ID"] = case
        for name in ("k6", "chromium") if is_browser else ("k6",):
            require(sha(self.state["tools"][name]["path"]) == self.state["tools"][name]["sha256"], "E2E_STIMULUS_TOOL_CHANGED")
        child = Child()
        with attempt.open("x", encoding="ascii") as marker:
            marker.write("attempted")
        try:
            if is_browser:
                result = campaign_browser.execute(self.r, output)
                code = result["exit"]
                child.code, child.cleanup = code, result["cleanup"]
                child.sandbox, child.renderer_observed = result["sandbox"], result["renderer_observed"]
                child.sandbox_proof = result["witnesses"]
            else:
                code = child.execute([self.state["tools"]["k6"]["path"], "run", "--quiet", "--no-usage-report", "--address",
                                  "127.0.0.1:0", "--config", str(self.root / "private/options.json"),
                                  str(ROOT / "ops/tests/k6/scenarios" / script)], output=output, env=env,
                                 timeout=record["timeout_seconds"], browser=is_browser)
            path = output / "private/summary.json"
            require(path.is_file() and not path.is_symlink() and not path.parent.is_symlink(), "E2E_CASE_SUMMARY_PATH")
            with path.open("rb") as stream:
                value = evidence.accept_summary(stream.read(evidence.SUMMARY_LIMIT + 1), self.state["run_id"], case, code)
            require(child.cleanup is True and (not is_browser or child.sandbox is True), "E2E_CHILD_EVIDENCE")
            self.publish_case(case, value)
            self.state["completed_cases"].append(case)
        except Rejected as error:
            self.state["browser_diagnostic"] = getattr(error, "browser_diagnostic", None) if is_browser else None
            raise
        finally:
            if is_browser:
                # The launcher validates and saves evidence before propagating
                # a nonzero container exit. Retain actual child cleanup facts.
                observed = self.state.get("browser_container", {}).get("result")
                if observed is not None:
                    child.code, child.cleanup = observed["exit"], observed["cleanup"]
                    child.sandbox, child.renderer_observed = observed["sandbox"], observed["renderer_observed"]
                    child.sandbox_proof = observed["witnesses"]
                self.state["chromium_diagnostic"] = browser_profile.private_log_diagnostic(output / "private/chromium.log")
            process = {"exit": child.code, "cleanup": child.cleanup,
                "sandbox": child.sandbox if is_browser else None, "renderer_observed": child.renderer_observed if is_browser else None}
            if case not in self.state["completed_cases"]:
                process.update(read_failed_summary(output / "private/summary.json", self.state["run_id"], case)
                               if child.cleanup is True else {"summary_status": "not_collected"})
            self.state["process_evidence"][case] = process
            self.r.save()
            self.publish("processes.json", self.state["process_evidence"])

    def restore(self):
        if self.state["certificate_phase"] == "good" or self.state["restoration"] == "PASS":
            return
        self.state["restoration"] = "PENDING"
        self.r.save()
        self.certificate("restored")
        self.scenario("auth-envoy-san-restored")
        self.state["restoration"] = "PASS"
        self.r.save()

    def bootstrap(self):
        values = self.state["values"]
        self.session_replica = replica = SessionReplica(self.state["run_id"], values["database"])
        root = self.root / "private/pki"
        root.mkdir(mode=0o700)
        pki.generate(root, self.state["run_id"], lambda args: self.r.cli("openssl", args))
        (root / "ca-bundle.crt").write_bytes(Path("/etc/ssl/certs/ca-certificates.crt").read_bytes() + b"\n" + (root / "ca.crt").read_bytes())
        configs, provider, target = inputs.configuration(values)
        write_json(self.root / "private/target.json", target)
        write_json(self.root / "private/options.json", {})
        self.r.create_cluster()
        self.create(yaml.safe_load((HERE / "namespace.yaml").read_text()))
        ca = (root / "ca.crt").read_bytes()
        self.secret("transport-ca", {"ca.crt": ca})
        self.secret("postgres-credentials", {"password": values["database_password"]})
        for role in ("postgres", "openfga", "localstack"):
            self.secret(role + "-server-tls", {"tls.crt": (root / (role + ".crt")).read_bytes(),
                                               "tls.key": (root / (role + ".key")).read_bytes()})
        uri = "postgres://postgres:" + values["database_password"] + "@postgres." + NS + ".svc.cluster.local:5432/openfga_dev?sslmode=verify-full&sslrootcert=/tls/ca.crt"
        self.secret("openfga-datastore", {"uri": uri})
        self.backings(session_replica=replica, postgres_create=True, openfga_create=False, localstack_create=False)
        self.rollout("postgres")
        self.sql("CREATE DATABASE " + values["database"] + ";")
        self.sql("CREATE DATABASE openfga_dev;")
        replica.create_database(self.sql)
        self.state["database_oid"] = int(self.sql("SELECT oid FROM pg_database WHERE datname='" + values["database"] + "'"))
        self.r.save()
        self.backings(postgres_create=False, openfga_create=True, localstack_create=True, network_create=False)
        self.rollout("openfga")
        self.rollout("localstack")
        self.create({"apiVersion": "v1", "kind": "ConfigMap", "metadata": {"name": "kind-local-fga-model"},
                     "data": {"model.fga": (ROOT / "ops/openfga/model.fga").read_text()}})
        self.backings(postgres_create=False, openfga_create=False, localstack_create=False, network_create=False,
                      openfga_initialize=True, localstack_initialize=True)
        for name in ("kind-local-fga-initialize", "kind-local-sqs-initialize"):
            self.r.k(["wait", "--for=condition=complete", "job/" + name, "--timeout=180s"], timeout=200)
        fga = strict_json(self.r.k(["logs", "job/kind-local-fga-initialize"]))
        store, model = fga["store"]["id"], fga["model"]["authorization_model_id"]
        require(all(re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{26}", value) for value in (store, model)), "E2E_FGA_RESULT")
        self.create({"apiVersion": "v1", "kind": "ConfigMap", "metadata": {"name": "local-full-fga"}, "data": {"store-id": store, "model-id": model}})
        for role in ("iam", "oauth", "idp", "envoy", "ext-authz", "late-check"):
            leaf = {"iam": "iam-service", "envoy": "envoy-mesh"}.get(role, role)
            self.secret(self.state["name"] + "-" + role + "-tls", {
                "ca.crt": ca, "ca-bundle.crt": (root / "ca-bundle.crt").read_bytes(),
                leaf + ".crt": (root / (leaf + ".crt")).read_bytes(), leaf + ".key": (root / (leaf + ".key")).read_bytes()})
        iam_config = ('session_checkpoint_token_env = "K6_IAM_CHECKPOINT_TOKEN"\n'
                      f'internal_service_token = "{replica.liveness_token}"\n' + configs["iam"])
        self.secret(self.state["name"] + "-iam-config", {"development.toml": iam_config, "k6.toml": iam_config,
                    "platform-signing.pem": (root / "platform-signing.pem").read_bytes(), "platform-signing.pub": (root / "platform-signing.pub").read_bytes()})
        database_url = "postgresql://postgres:" + values["database_password"] + "@postgres." + NS + ".svc.cluster.local:5432/" + values["database"] + "?sslmode=verify-full&sslrootcert=/app/certs/ca.crt"
        self.secret(self.state["name"] + "-iam-env", {"DATABASE_URL": database_url, **replica.iam_secret()})
        identity = (root / "ext-authz.crt").read_bytes() + (root / "ext-authz.key").read_bytes()
        self.secret(self.state["name"] + "-replica-env", replica.gateway_secret(identity))
        self.secret(self.state["name"] + "-oauth-config", {"development.toml": configs["oauth"], "k6.toml": configs["oauth"]})
        self.secret(self.state["name"] + "-idp-config", {"provider.json": json.dumps(provider)})
        fixtures = json.loads((HERE / "fixtures.lock.json").read_text())
        images = {role: item["tag"] for role, item in self.state["images"]["roles"].items()} | {role: fixtures[role] for role in ("envoy", "idp")}
        for role in ("iam", "oauth", "ext-authz"):
            self.r.cli("kind", ["load", "docker-image", "--name", self.state["cluster"], images[role]], timeout=180)
        envoy = yaml.safe_load((HERE / "config/envoy.yaml").read_text())
        late_check.add_listener(envoy)
        objects = self.render(HERE / "chart", {
            "name": self.state["name"], "runId": self.state["run_id"], "ciEnabled": True, "images": images,
            "envoySource": json.dumps(envoy), "lateCheckSource": (HERE / "late_check.py").read_text(),
            "simulatorSource": (ROOT / "ops/tests/support/oauth-idp-simulator.py").read_text()})
        require(len(objects) == 15, "E2E_FIXTURE_INVENTORY")
        iam, = [item for item in objects if item["kind"] == "Deployment"
                and item["metadata"]["name"] == self.state["name"] + "-iam"]
        self.direct_iam = direct = deepcopy(iam)
        direct["metadata"]["name"] += "-direct"
        direct["spec"]["selector"]["matchLabels"]["aiforall.dev/k6-role"] = "iam-direct"
        template = direct["spec"]["template"]
        template["metadata"]["labels"]["aiforall.dev/k6-role"] = "iam-direct"
        container, = template["spec"]["containers"]
        container["env"] = [entry for entry in container["env"] if entry["name"] != "IAM_AUTH__MESH__TRUSTED_GATEWAY_SAN"]
        # The direct verifier refreshes its own JWKS over the pod's loopback.
        # The public issuer URL belongs to the runner, not the pod.
        container["env"].extend([
            {"name": "IAM_SERVER__HOST", "value": "127.0.0.1"},
            {"name": "IAM_SERVER__TLS_ENABLED", "value": "false"},
            {"name": "IAM_JWT__JWKS_URL", "value": "http://127.0.0.1:8080/iam/.well-known/jwks.json"},
        ])
        container["ports"] = [{"name": "http", "containerPort": 8080}]
        container["readinessProbe"] = {"exec": {"command": [
            "curl", "--silent", "--fail", "--max-time", "1", "http://127.0.0.1:8080/iam/health",
        ]}, "periodSeconds": 2, "timeoutSeconds": 2}
        for item in objects:
            self.create(item)
        for role in ("idp", "oauth", "iam"):
            self.rollout(self.state["name"] + "-" + role)
        replica.subscribe(self.sql)
        for role in ("ext-authz", "late-check", "envoy"):
            self.rollout(self.state["name"] + "-" + role)
        self.state["edge_identity"] = list(self.identity())
        self.r.save()
        self.forward("envoy", 18443, 10000)
        self.forward("envoy", 19901, 9901)
        self.forward("idp", 18445, 8080)
        self.wait_endpoint()
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if replica.observed_marker(self.sql):
                self.state["replica_bootstrap"] = True
                self.r.save()
                return
            time.sleep(0.25)
        raise Refused("E2E_REPLICA_CHECKPOINT_NOT_APPLIED")

    def admit_services(self):
        _, raw = self.http("http://127.0.0.1:19901/config_dump")
        dumps = [item for item in strict_json(raw)["configs"]
                 if item.get("@type") == "type.googleapis.com/envoy.admin.v3.ClustersConfigDump"]
        require(len(dumps) == 1, "E2E_LOADED_CLUSTERS")
        clusters = [item["cluster"] for item in dumps[0]["static_clusters"] if item["cluster"]["name"] == "iam"]
        require(len(clusters) == 1 and clusters[0]["load_assignment"]["endpoints"][0]["lb_endpoints"][0]
                ["endpoint"]["address"]["socket_address"]["address"] == self.state["name"] + "-iam." + NS + ".svc.cluster.local",
                "E2E_LOADED_IAM_DESTINATION")
        pod = self.pod("iam")["metadata"]["name"]
        jwks = []
        for direct in (True, False):
            args = ["exec", pod, "-c", "iam", "--", "curl", "--silent", "--show-error", "--fail", "--max-time", "10",
                    "--noproxy", "*", "--cacert", "/app/certs/ca.crt", "--cert", "/app/certs/iam-service.crt",
                    "--key", "/app/certs/iam-service.key"]
            if direct:
                args += ["--resolve", "iam." + NS + ".svc.cluster.local:8443:127.0.0.1"]
            args += ["https://iam." + NS + ".svc.cluster.local:8443/iam/.well-known/jwks.json"]
            jwks.append(strict_json(self.r.k(args)))
        keys = [key for key in jwks[0]["keys"] if key.get("kty") == "RSA"]
        require(len(keys) == 1 and keys[0].get("kid") == self.state["run_id"] and
                len([key for key in jwks[1]["keys"] if all(key.get(name) == keys[0].get(name)
                                                         for name in ("kid", "n", "e"))]) == 1,
                "E2E_JWKS_INCOMPATIBLE")
        require(self.state.get("replica_bootstrap") is True, "E2E_REPLICA_BOOTSTRAP_REQUIRED")
        self.publish("admission.json", {"loaded_iam_destination": True, "jwks_compatible": True,
                                        "replica_checkpoint_applied": True})

    def run_cases(self):
        self.admit_services()
        san_good = san_bad = san_restored = None
        correlation = {"san_delta": 1, "iam_delta": 7,
                       "ext_authz_delta": 4, "provider_delta": 3, "pkce_delta": 3,
                       "users_delta": 1, "outbox_delta": 0}
        for case, record in evidence.CATALOG.items():
            if case == evidence.LINK_EMAIL_CASE:
                correlation["link_email"] = self.run_link_email()
            elif record["lifecycle"] == "standard":
                self.scenario(case)
            elif case == "auth-envoy-san-good":
                san_good, san_bad, san_restored = self.run_san()
            elif record["lifecycle"] == "oauth":
                self.run_oauth()
        require(self.state["completed_cases"] == list(evidence.CATALOG), "E2E_INCOMPLETE_CASES")
        require(all(value is not None for value in (san_good, san_bad, san_restored)), "E2E_SAN_MISSING")
        correlation["restoration"] = self.state["restoration"]
        self.publish("correlation.json", correlation)

    def qualify_session_recovery(self):
        self.create(self.direct_iam)
        self.rollout(self.state["name"] + "-iam-direct")
        self.forward("iam-direct", DIRECT_PORT, 8080)
        self.forward("envoy", late_check.PORT, late_check.PORT)
        recovery = HostedRecovery(self, self.session_replica)
        admission = {"loaded_iam_destination": True, "jwks_compatible": True, "replica_checkpoint_applied": True}
        admission["session_recovery_nominal"], fresh = recovery.nominal()
        self.publish("admission.json", admission)
        admission["late_check"] = recovery.late_response(fresh)
        self.publish("admission.json", admission)
        admission["session_recovery_degraded"] = recovery.degraded()
        self.publish("admission.json", admission)
        for case, rotation_first in (("renewal_first", True), ("reset_first", False)):
            admission[case] = recovery.ordered_race(rotation_first)
            self.publish("admission.json", admission)
        if RECOVERY_ROLE in self.state["images"]["roles"]:
            snapshot_key = create_monolith(self)
            admission["monolith"] = recovery.companions(snapshot_key, nested=True)
            self.publish("admission.json", admission)
            create_standalone(self, snapshot_key)
            admission["standalone_services"] = recovery.companions(snapshot_key, nested=False)
            self.publish("admission.json", admission)

    def run_san(self):
        self.scenario("auth-envoy-san-good")
        good = self.snapshot()
        try:
            self.state["certificate_phase"] = "uncertain"
            self.r.save()
            self.certificate("bad")
            self.scenario("auth-envoy-san-bad")
            bad = self.snapshot()
        finally:
            self.restore()
        restored = self.snapshot()
        evidence.accept_san(good, bad, restored, [tuple(self.state["edge_identity"])] * 3)
        return good, bad, restored

    def run_oauth(self):
        # Warm the real protected route outside the measurement window. A 403
        # alone is not enough: exactly one real ext-authz request must be seen.
        start = self.snapshot()
        status, _ = self.http("https://127.0.0.1:18443/iam/api/auth/gitlab/relink-start")
        finish = self.snapshot()
        require(status == 403 and evidence.delta(start, finish, "cluster.ext_authz.upstream_rq_total") == 1,
                "E2E_AUTHZ_ADMISSION")
        before = self.snapshot()
        _, raw = self.http("https://localhost:18445/observations")
        provider_before = strict_json(raw)
        sql = "SELECT json_build_object('users',(SELECT count(*) FROM users),'outbox',(SELECT count(*) FROM rustycog_outbox_events))"
        db_before = strict_json(self.sql(sql, self.state["values"]["database"]))
        self.scenario("auth-oauth-browser")
        _, raw = self.http("https://localhost:18445/observations")
        provider_after = strict_json(raw)
        db_after = strict_json(self.sql(sql, self.state["values"]["database"]))
        after = self.snapshot()
        evidence.accept_oauth(before, after, provider_before, provider_after, db_before, db_after, self.state["run_id"])

    def link_email_state(self):
        run = self.state["run_id"]
        database = self.state["values"]["database"]
        require(re.fullmatch(r"[a-f0-9]{32}", run) and database == "k6_" + run, "E2E_LINK_EMAIL_DB_SCOPE")
        # Independent readback through the same owned Postgres socket as the
        # browser counters. No row values, subjects, or token columns leave SQL.
        query = f"""
            WITH examples(kind) AS (VALUES ('attested'), ('false'), ('omitted')),
            identities AS (
                SELECT kind, 's502-' || kind || '-{run}' AS username,
                       '{run}-owner-' || kind AS original_subject,
                       '{run}-link-' || kind AS linked_subject
                FROM examples
            ), observed AS (
                SELECT kind, u.id,
                    (SELECT count(*) FROM user_emails e WHERE e.user_id = u.id) AS emails,
                    (SELECT count(*) FROM user_emails e WHERE e.user_id = u.id
                        AND e.email = original_subject || '@example.invalid'
                        AND e.is_primary AND e.is_verified) AS primary_preserved,
                    (SELECT count(*) FROM user_emails e WHERE e.user_id = u.id
                        AND e.email = linked_subject || '@example.invalid' AND NOT e.is_primary
                        AND e.is_verified = (kind = 'attested')) AS secondary,
                    (SELECT count(*) FROM provider_tokens p WHERE p.user_id = u.id) AS methods,
                    (SELECT count(*) FROM provider_tokens p WHERE p.user_id = u.id
                        AND p.provider = 'gitlab' AND p.provider_user_id = original_subject) AS existing_method,
                    (SELECT count(*) FROM provider_tokens p WHERE p.user_id = u.id
                        AND p.provider = 'github' AND p.provider_user_id = linked_subject) AS new_method
                FROM identities LEFT JOIN users u USING (username)
            )
            SELECT json_build_object(
                'users', (SELECT count(*) FROM users),
                'outbox', (SELECT count(*) FROM rustycog_outbox_events),
                'owners', count(id),
                'primary_preserved', count(*) FILTER (WHERE primary_preserved = 1),
                'attested_secondary', count(*) FILTER (WHERE kind = 'attested' AND secondary = 1),
                'unproved_secondary', count(*) FILTER (WHERE kind <> 'attested' AND secondary = 1),
                'existing_method', count(*) FILTER (WHERE existing_method = 1),
                'new_method', count(*) FILTER (WHERE new_method = 1),
                'extra_rows', count(*) FILTER (WHERE id IS NOT NULL AND (emails <> 2 OR methods <> 2))
            ) FROM observed
        """
        return strict_json(self.sql(query, database))

    def run_link_email(self):
        before = self.snapshot()
        _, raw = self.http("https://localhost:18445/observations")
        provider_before = strict_json(raw)
        db_before = self.link_email_state()
        self.scenario(evidence.LINK_EMAIL_CASE)
        _, raw = self.http("https://localhost:18445/observations")
        provider_after = strict_json(raw)
        db_after = self.link_email_state()
        after = self.snapshot()
        return evidence.accept_link_email(before, after, provider_before, provider_after,
                                          db_before, db_after, self.state["run_id"])


def prepare(directory, cli_directory, browser_directory, image_directory):
    hosted()
    harness = os.environ["GITHUB_SHA"]
    if premerge_job(os.environ):
        harness = os.environ.get("K6_PR_TARGET_SHA", "")
        require(re.fullmatch(r"[a-f0-9]{40}", harness)
                and command(["git", "rev-parse", "HEAD"], cwd=ROOT) == harness, "E2E_PR_HARNESS_SHA")
    os.umask(0o077)
    root = new_output(str(Path(directory).resolve()))
    (root / "public").mkdir(mode=0o700)
    run = uuid.uuid4().hex
    cli_directory = Path(cli_directory).resolve()
    hashes = strict_json((cli_directory / "hashes.json").read_bytes())
    tools = {name: {"path": str(cli_directory / name), "sha256": hashes[name]} for name in ("kind", "helm", "kubectl")}
    for name in ("openssl", "certutil", "docker", "ldd"):
        path = shutil.which(name)
        require(path is not None, "E2E_HOST_TOOL_MISSING")
        tools[name] = {"path": path, "sha256": sha(path)}
    tools.update(browser_profile.tools(browser_directory))
    images_root = Path(image_directory).resolve()
    images = strict_json((images_root / "verified/provenance.json").read_bytes())
    if premerge_job(os.environ):
        require(images["target_sha"] == harness, "E2E_PR_IMAGE_SHA")
    require(sha(images_root / "verified/images.tar.gz") == images["archive_sha256"], "E2E_IMAGE_ARCHIVE_CHANGED")
    state = {"version": 1, "run_id": run, "name": "k6-" + run, "job_run_id": os.environ["GITHUB_RUN_ID"],
        "job_attempt": os.environ["GITHUB_RUN_ATTEMPT"], "cluster": "k6-e2e-" + os.environ["GITHUB_RUN_ID"] + "-" + os.environ["GITHUB_RUN_ATTEMPT"],
         "kubeconfig": str(root / "private/kubeconfig"), "tools": tools, "values": inputs.fresh(run), "images": images,
         "harness_sha": harness,
        "host": socket.gethostname(), "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
        "resources": [], "processes": [], "completed_cases": [], "process_evidence": {}, "certificate_phase": "good",
        "restoration": "NOT_REQUIRED", "verdict": "NOT_RUN", "cleanup": {"status": "NOT_RUN"}, "volumes": []}
    runtime = Runtime(root, state)
    runtime.save()
    Campaign(runtime).publish("selection.json", evidence.selection_receipt(run, harness, images["target_sha"]))
    state["before_containers"] = sorted(runtime.container_ids())
    state["before_volumes"] = runtime.docker(["volume", "ls", "--format", "{{.Name}}"]).decode().splitlines()
    state["known_images"] = sorted(runtime.image_ids())
    runtime.save()
    state["browser_preflight"] = browser_profile.preflight(tools, runtime.cli)
    runtime.save()
    campaign_browser.prepare(runtime, browser_directory)
    try:
        runtime.docker(["image", "load", "--input", str(images_root / "verified/images.tar.gz")], timeout=180)
    finally:
        runtime.observe_images()
    for role, item in images["roles"].items():
        actual = strict_json(runtime.docker(["image", "inspect", item["tag"]]))[0]
        require(actual["Id"] == item["image_id"] and actual["Architecture"] == "amd64" and actual["Os"] == "linux", "E2E_LOADED_IMAGE_ID")
        labels = actual["Config"]["Labels"]
        require(labels["org.opencontainers.image.revision"] == images["target_sha"]
                and labels["aiforall.dev.sdk-revision"] == images["sdk_sha"], "E2E_LOADED_IMAGE_PROVENANCE")
    return Campaign(runtime)


def load(directory):
    hosted()
    root = Path(directory).resolve()
    private_path(root, directory=True)
    state = strict_json((root / "private/state.json").read_bytes())
    require(state["version"] == 1 and state["job_run_id"] == os.environ["GITHUB_RUN_ID"]
            and state["job_attempt"] == os.environ["GITHUB_RUN_ATTEMPT"], "E2E_STATE_OWNER")
    return Campaign(Runtime(root, state))


def finish(campaign):
    state = campaign.state
    browser_cleanup = True
    try:
        campaign_browser.clean(campaign.r)
    except Exception:
        browser_cleanup = False
        state["verdict"] = "FAIL"
        campaign.r.save()
    try:
        campaign.restore()
    except Exception:
        state["restoration"] = "FAIL"
        state["verdict"] = "FAIL"
        campaign.r.save()
    try:
        cleanup = campaign.r.close()
    except Exception:
        cleanup = {"status": "FAIL", "owned_running": None, "errors": ["CLEANUP_UNPROVEN"]}
        state["cleanup"] = cleanup
    if not browser_cleanup:
        cleanup.update(status="FAIL", owned_running=None)
        cleanup.setdefault("errors", []).append("BROWSER_CONTAINER_CLEANUP_UNPROVEN")
        state["cleanup"] = cleanup
    cases = campaign.root / "private" / "cases"
    attempts = {path.name.removesuffix(".attempt") for path in cases.glob("*.attempt")} if not cases.is_symlink() else set()
    if cases.is_symlink():
        cleanup.update(status="FAIL", owned_running=None)
        cleanup.setdefault("errors", []).append("CHILD_CASE_DIRECTORY_UNPROVEN")
    if any(state.get("process_evidence", {}).get(case, {}).get("cleanup") is not True for case in attempts):
        cleanup.update(status="FAIL", owned_running=None)
        cleanup.setdefault("errors", []).append("CHILD_CLEANUP_UNPROVEN")
        state["cleanup"] = cleanup
    if cleanup["status"] != "PASS":
        state["verdict"] = "FAIL"
    state["cleanup"] = cleanup
    campaign.publish("cleanup.json", cleanup)
    browser_state = state.get("browser_container", {})
    campaign.publish("browser.json", {"launcher": "docker-host-network-disposable-runner", "image_id": browser_state.get("image_id"),
        "profile": browser_state.get("profile_facts"), "policy": browser_state.get("policy"),
        "cleanup": browser_state.get("cleanup"), "container_exit": browser_state.get("container_exit"), "result": browser_state.get("result")})
    with (campaign.public / "browser-docker-resources.jsonl").open("w", encoding="utf-8") as target:
        for item in campaign_browser.browser_receipts(campaign.r):
            target.write(json.dumps(item, sort_keys=True) + "\n")
    records = campaign.r.receipts()
    with (campaign.public / "docker-resources.jsonl").open("w", encoding="utf-8") as target:
        for item in records:
            target.write(json.dumps(item, sort_keys=True) + "\n")
    campaign.publish("report.json", {"version": 1, "run_id": state["run_id"], "verdict": state["verdict"],
        "diagnostic": state.get("diagnostic"), "browser_diagnostic": state.get("browser_diagnostic"),
        "chromium_diagnostic": state.get("chromium_diagnostic"), "browser_preflight": state.get("browser_preflight"),
        "restoration": state["restoration"], "completed_cases": state["completed_cases"], "cleanup": cleanup["status"],
        "stage": state.get("stage"), "resources": state.get("resources", [])})
    campaign.publish("provenance.json", {"harness_sha": state.get("harness_sha") or os.environ["GITHUB_SHA"], "target_sha": state["images"]["target_sha"],
        "sdk_sha": state["images"]["sdk_sha"], "packaging_sha": state["images"]["harness_sha"],
        "images": {role: value["image_id"] for role, value in state["images"]["roles"].items()},
        "tools": {name: {key: value for key, value in pin.items() if key != "path"} for name, pin in state["tools"].items()}})
    campaign.r.save()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run", "clean"))
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--cli-directory", type=Path)
    parser.add_argument("--browser-directory", type=Path)
    parser.add_argument("--image-directory", type=Path)
    args = parser.parse_args()
    os.umask(0o077)
    campaign = None
    try:
        if args.action == "clean":
            if not (args.directory / "private/state.json").exists():
                print("No E2E runtime was admitted; no cleanup authority acquired")
                return 0
            campaign = load(args.directory)
            finish(campaign)
            return 0 if campaign.state["cleanup"]["status"] == "PASS" else 1
        require(all(value is not None for value in (args.cli_directory, args.browser_directory, args.image_directory)), "E2E_INPUTS_REQUIRED")
        campaign = prepare(args.directory, args.cli_directory, args.browser_directory, args.image_directory)

        def interrupted(_signal, _frame):
            raise Refused("E2E_INTERRUPTED")

        signal.signal(signal.SIGTERM, interrupted)
        campaign.bootstrap()
        campaign.run_cases()
        campaign.qualify_session_recovery()
        campaign.state["verdict"] = "PASS"
    except BaseException as error:
        if campaign is None and (args.directory / "private/state.json").exists():
            campaign = load(args.directory)
        if campaign:
            campaign.state["verdict"] = "FAIL"
            campaign.state["diagnostic"] = error.code if isinstance(error, Rejected) else str(error) if isinstance(error, (Refused, evidence.EvidenceError)) else "E2E_UNCLASSIFIED_" + type(error).__name__
            campaign.r.save()
    finally:
        if campaign and args.action == "run":
            try:
                finish(campaign)
            except Exception:
                campaign.state["verdict"] = "FAIL"
                campaign.r.save()
    if campaign and campaign.state["verdict"] == "PASS" and campaign.state["cleanup"]["status"] == "PASS":
        checks = sum(len(record["checks"]) for record in evidence.CATALOG.values())
        print(f"Ubuntu k6 E2E PASS; {len(evidence.CATALOG)} cases, {checks} checks and parent deltas; cleanup PASS")
        return 0
    print("Three-scenario Ubuntu E2E failed or incomplete; no compiler fallback")
    return 1


if __name__ == "__main__":
    sys.exit(main())
