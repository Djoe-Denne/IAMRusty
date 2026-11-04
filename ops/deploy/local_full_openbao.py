"""In-memory local coffer wiring; selected private IAM sources stay private.

This is not an activation authority: it never reads a live Secret, qualifies a
source's runtime provenance, or applies objects to an existing environment.
"""
import copy
import base64
import json
from pathlib import Path
import re
import tomllib

OVERLAY = Path(__file__).parent / "apps/overlays/local-full"
NAMESPACE = "aiforall-local-full"
ADDRESS = "https://openbao.aiforall-local-full.svc.cluster.local:8443"
PYTHON = "python:3.12.9-alpine3.21@sha256:28b8a72c4e0704dd2048b79830e692e94ac2d43d30c914d54def6abf74448a4e"
BAO = "openbao/openbao:2.6.2@sha256:11fd73a2102cda9c55d5d881a8c3210303146a7ec1e8ac76f526e175c6d24641"
SECURITY = {"allowPrivilegeEscalation": False, "readOnlyRootFilesystem": True, "capabilities": {"drop": ["ALL"]}}
POD_SECURITY = {"runAsNonRoot": True, "runAsUser": 1000, "runAsGroup": 1000, "fsGroup": 1000,
                "seccompProfile": {"type": "RuntimeDefault"}}
RESOURCES = {"requests": {"cpu": "10m", "memory": "32Mi"}, "limits": {"cpu": "200m", "memory": "128Mi"}}


def resource(kind, name, namespace=NAMESPACE, **values):
    version = "apps/v1" if kind == "Deployment" else "batch/v1" if kind == "Job" else "networking.k8s.io/v1" if kind == "NetworkPolicy" else "v1"
    return {"apiVersion": version, "kind": kind, "metadata": {"name": name, "namespace": namespace}, **values}


def mount(name, path, readonly=True):
    return {"name": name, "mountPath": path, "readOnly": readonly}


def identity(role):
    return {"name": "openbao-identity", "projected": {"defaultMode": 0o440, "sources": [
        {"serviceAccountToken": {"path": "jwt", "audience": "openbao-" + role, "expirationSeconds": 600}}]}}


def snapshot(pod, role, account, scripts, config_volume="config"):
    pod["serviceAccountName"] = account
    pod["automountServiceAccountToken"] = False
    pod["securityContext"] = copy.deepcopy(POD_SECURITY)
    volumes = pod.setdefault("volumes", [])
    config = next(v for v in volumes if v["name"] == config_volume)
    # Preserve the selected source's privacy; only the Memory snapshot is mounted
    # by the application. A combined GitLab template is still a private Secret.
    volumes.extend([identity(role), {"name": "private-config", "emptyDir": {"medium": "Memory", "sizeLimit": "1Mi"}},
                    {"name": "openbao-ca", "configMap": {"name": "openbao-public-ca"}},
                    {"name": "openbao-scripts", "configMap": {"name": scripts}}])
    init = {"name": "openbao-snapshot", "image": PYTHON,
            "command": ["python", "-B", "/scripts/openbao-workload.py", role],
            "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
            "volumeMounts": [mount("openbao-identity", "/run/identity"), mount("openbao-ca", "/run/openbao-ca"),
                             mount("openbao-scripts", "/scripts"), mount(config["name"], "/template"),
                             mount("private-config", "/runtime", False)]}
    pod.setdefault("initContainers", []).insert(0, init)
    for container in pod["containers"]:
        for item in container.get("volumeMounts", []):
            if item["name"] == config_volume and (item["mountPath"].endswith("development.toml") or item["mountPath"] == "/app/config"):
                item["name"] = "private-config"
                item["readOnly"] = True
        container.setdefault("volumeMounts", []).append(mount("openbao-ca", "/run/openbao-ca"))


def github_seed_job(run_id, script_name):
    """Same bounded non-root seed for initial deployment and a new local epoch."""
    pod = {"serviceAccountName": "github-seed", "automountServiceAccountToken": False,
        "restartPolicy": "Never", "securityContext": copy.deepcopy(POD_SECURITY),
        "containers": [{"name": "seed", "image": PYTHON, "command": ["python", "-B", "/scripts/openbao-workload.py", "github-seed"],
            "resources": copy.deepcopy(RESOURCES), "securityContext": copy.deepcopy(SECURITY),
            "volumeMounts": [mount("scripts", "/scripts"), mount("openbao-ca", "/run/openbao-ca"),
                mount("openbao-identity", "/run/identity"), mount("private-inputs", "/inputs")]}],
        "volumes": [identity("github-seed"), {"name": "scripts", "configMap": {"name": script_name}},
            {"name": "openbao-ca", "configMap": {"name": "openbao-public-ca"}},
            {"name": "private-inputs", "secret": {"secretName": "github-seed-inputs-" + run_id, "defaultMode": 0o440}}]}
    return resource("Job", "github-openbao-seed", spec={"backoffLimit": 0, "activeDeadlineSeconds": 240,
        "ttlSecondsAfterFinished": 300, "template": {"metadata": {"labels": {"app.kubernetes.io/name": "github-seed"}}, "spec": pod}})


def iam_template(objects):
    """Find the single actually mounted IAM source, never an unused base CM.

    Returns (pod, volume, source object, text). Only operates on in-memory
    objects; it does not read a live Secret or attest deployment provenance.
    """
    try:
        deployments = [o for o in objects if o["kind"] == "Deployment" and o["metadata"]["name"] == "iam"]
        if len(deployments) != 1:
            raise ValueError
        pod = deployments[0]["spec"]["template"]["spec"]
        containers = [c for c in pod["containers"] if c["name"] == "iam"]
        if len(containers) != 1:
            raise ValueError
        mounts = [m for m in containers[0]["volumeMounts"]
                  if m["mountPath"] in {"/app/config", "/app/config/development.toml"}]
        if len(mounts) != 1:
            raise ValueError
        volumes = [v for v in pod["volumes"] if v["name"] == mounts[0]["name"]]
        if len(volumes) != 1:
            raise ValueError
        volume = volumes[0]
        if set(volume) - {"name"} not in ({"secret"}, {"configMap"}):
            raise ValueError
        private = "secret" in volume
        reference = volume["secret" if private else "configMap"]
        name = reference["secretName" if private else "name"]
        sources = [o for o in objects if o["kind"] == ("Secret" if private else "ConfigMap")
                   and o["metadata"]["name"] == name and o["metadata"].get("namespace") == NAMESPACE]
        if (len(sources) != 1 or "items" in reference or (private and mounts[0].get("readOnly") is not True)
                or mounts[0].get("subPath", "development.toml") != "development.toml"):
            raise ValueError
        source = sources[0]
        if "development.toml" in source.get("stringData", {}):
            raise ValueError
        text = source["data"]["development.toml"]
        if not isinstance(text, str):
            raise ValueError
        if private:
            text = base64.b64decode("".join(text.split()), validate=True).decode("utf-8")
        if not isinstance(text, str) or len(text.encode("utf-8")) > 1048576:
            raise ValueError
        return pod, volume, source, text
    except (KeyError, TypeError, ValueError, UnicodeError):
        raise ValueError("SOURCE_IAM_UNAVAILABLE: unique selected IAM source required (values redacted)") from None


def compose_github_template(template, private=True):
    """Add exactly one GitHub table and prove semantic equality of the preimage.

    Private preimages must be complete GitLab-only configurations. The historical
    public empty-registry profile remains separate and is never a migration.
    """
    github = {"id": "github", "base_url": "https://github-connect:8443/github-connect",
              "hmac_secret": "__GITHUB_OPENBAO_HMAC__", "pkce_supported": False,
              "redirect_uris": ["http://127.0.0.1:18080/iam/api/auth/github/callback",
                                "http://127.0.0.1:18080/iam/api/auth/github/relink-callback"]}
    try:
        before = tomllib.loads(template)
        connectors = before["idp"]["connectors"]
        if not isinstance(connectors, list) or re.search(r"__[A-Z][A-Z0-9_]*__", template):
            raise ValueError
        if private:
            if len(connectors) != 1:
                raise ValueError
            connector = connectors[0]
            if (set(connector) != set(github) or not isinstance(connector["id"], str)
                    or connector["id"].lower() != "gitlab"
                    or connector["base_url"] != "https://gitlab-connect:8443/gitlab-connect"
                    or not isinstance(connector["hmac_secret"], str) or len(connector["hmac_secret"].encode()) < 16
                    or type(connector["pkce_supported"]) is not bool or not connector["pkce_supported"]
                    or connector["redirect_uris"] != ["http://127.0.0.1:18080/iam/api/auth/gitlab/callback",
                                                     "http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback"]):
                raise ValueError
            base = template
        else:
            if connectors or len(re.findall(r"(?m)^connectors = \[\]\s*$", template)) != 1:
                raise ValueError
            base = re.sub(r"(?m)^connectors = \[\]\s*$", "", template, count=1)
        # An explicit dotted table switches back to the root even when the last
        # preimage section was [queue.queues]. Parsing and equality below are the
        # authority, not the textual placement alone.
        rendered = base + "\n[[idp.connectors]]\n" + "\n".join(
            key + " = " + json.dumps(value) for key, value in github.items()) + "\n"
        after = tomllib.loads(rendered)
        added = after["idp"]["connectors"].pop()
        if added != github or after != before or rendered.count(github["hmac_secret"]) != 1:
            raise ValueError
        return rendered
    except (KeyError, TypeError, ValueError):
        # TOML errors may include private source excerpts: never propagate them.
        raise ValueError("invalid or ambiguous IAM connector preimage (values redacted)") from None


def configure(objects, run_id, github=False, client_id="offline-synthetic-client"):
    """Plan in isolation so a refused private preimage leaves all inputs intact."""
    planned = copy.deepcopy(objects)
    _configure(planned, run_id, github, client_id)
    for original, replacement in zip(objects, planned):
        original.clear()
        original.update(replacement)
    objects.extend(planned[len(objects):])


def _configure(objects, run_id, github=False, client_id="offline-synthetic-client"):
    if not isinstance(client_id, str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", client_id):
        raise ValueError("invalid GitHub client ID")
    if github:
        pod, selected_volume, config, template = iam_template(objects)
        combined = compose_github_template(template, private=config["kind"] == "Secret")
    scripts = next(o for o in objects if o["kind"] == "ConfigMap" and o["metadata"]["name"].startswith("local-full-bootstrap-scripts"))
    scripts["data"]["check-github-token.sh"] = (OVERLAY / "helpers/check-github-token.sh").read_text()
    script_name = scripts["metadata"]["name"]
    if not any(o["kind"] == "ServiceAccount" and o["metadata"]["name"] == "lazaret" for o in objects):
        objects.append(resource("ServiceAccount", "lazaret", automountServiceAccountToken=False))
    for namespace in (NAMESPACE, "aiforall-local-apparatus"):
        if not any(o["kind"] == "ConfigMap" and o["metadata"]["name"] == "openbao-public-ca"
                   and o["metadata"].get("namespace") == namespace for o in objects):
            objects.append(resource("ConfigMap", "openbao-public-ca", namespace, data={"ca.crt": ""}))
    for obj in objects:
        name = obj["metadata"]["name"]
        if obj["kind"] == "NetworkPolicy":
            if name == "allow-openbao":
                obj["spec"]["ingress"][0]["from"][0]["podSelector"]["matchExpressions"][0]["values"] = ["lazaret"] + (
                    ["github-seed", "iam", "github-connect"] if github else [])
        if obj["kind"] != "Deployment":
            continue
        pod = obj["spec"]["template"]["spec"]
        if name == "openbao":
            pod["securityContext"] = copy.deepcopy(POD_SECURITY)
        if name == "lazaret" and not any(c["name"] == "openbao-snapshot" for c in pod.get("initContainers", [])):
            snapshot(pod, "lazaret", "lazaret", script_name)
        if name == "apparatus-operator":
            pod.setdefault("volumes", []).append({"name": "openbao-ca", "configMap": {"name": "openbao-public-ca"}})
            for container in pod["containers"]:
                container.setdefault("volumeMounts", []).append(mount("openbao-ca", "/run/openbao-ca"))
                env = container.setdefault("env", [])
                for item in env:
                    if item["name"] in {"APPARATUS_VAULT_ADDR", "APPARATUS_VAULT_ADDR_NETWORK"}:
                        item["value"] = ADDRESS
                env.append({"name": "APPARATUS_VAULT_CA_CERT", "value": "/run/openbao-ca/ca.crt"})
    if not github:
        return
    for account in ("github-connect", "github-seed", "iam-openbao"):
        objects.append(resource("ServiceAccount", account, automountServiceAccountToken=False))
    iam = next(o for o in objects if o["kind"] == "Deployment" and o["metadata"]["name"] == "iam")
    pod = iam["spec"]["template"]["spec"]
    config["data"]["development.toml"] = (base64.b64encode(combined.encode()).decode()
        if config["kind"] == "Secret" else combined)
    snapshot(pod, "iam-hmac", "iam-openbao", script_name, selected_volume["name"])
    # Extend the actual IAM trust store, including a pre-existing GitLab proof CA.
    # Replacing that bundle or appending a duplicate SSL_CERT_FILE breaks GitLab.
    app = next(c for c in pod["containers"] if c["name"] == "iam")
    trust_env = [e for e in app.get("env", []) if e["name"] == "SSL_CERT_FILE"]
    source_trust = "/etc/ssl/certs/ca-certificates.crt"
    source_mounts = []
    if trust_env:
        if (len(trust_env) != 1 or set(trust_env[0]) != {"name", "value"}
                or not re.fullmatch(r"/[A-Za-z0-9_./-]+", trust_env[0]["value"])
                or ".." in trust_env[0]["value"].split("/")):
            raise ValueError("ambiguous IAM trust source")
        source_trust = trust_env[0]["value"]
        source_mounts = [copy.deepcopy(m) for m in app.get("volumeMounts", [])
                         if source_trust.startswith(m["mountPath"].rstrip("/") + "/")]
        if len(source_mounts) != 1 or not source_mounts[0].get("readOnly"):
            raise ValueError("IAM private trust source is not a unique read-only mount")
    pod["volumes"].append({"name": "iam-trust", "emptyDir": {"medium": "Memory", "sizeLimit": "1Mi"}})
    pod["initContainers"].append({"name": "trusted-roots", "image": pod["containers"][0]["image"],
        "imagePullPolicy": "Never", "command": ["sh", "-ec",
            "cat " + source_trust + " /run/openbao-ca/ca.crt > /trust/ca-bundle.crt; chmod 0444 /trust/ca-bundle.crt"],
        "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
        "volumeMounts": [mount("openbao-ca", "/run/openbao-ca"), mount("iam-trust", "/trust", False)] + source_mounts})
    app.setdefault("volumeMounts", []).append(mount("iam-trust", "/run/iam-trust"))
    if trust_env:
        trust_env[0]["value"] = "/run/iam-trust/ca-bundle.crt"
    else:
        app.setdefault("env", []).append({"name": "SSL_CERT_FILE", "value": "/run/iam-trust/ca-bundle.crt"})
    connector_config = (OVERLAY / "github-openbao/github.toml").read_text().replace("__GITHUB_CLIENT_ID__", client_id)
    objects.append(resource("ConfigMap", "github-openbao-config", data={"development.toml": connector_config,
        "agent.hcl": (OVERLAY / "github-openbao/agent.hcl").read_text()}))
    shared = [mount("github-config", "/app/config"), mount("openbao-ca", "/run/openbao-ca"),
              mount("connector-tls", "/run/connector-tls"), mount("github-token", "/run/github-openbao")]
    agent = {"name": "openbao-agent", "image": BAO, "restartPolicy": "Always",
             "command": ["bao", "agent", "-config=/agent/agent.hcl", "-log-level=warn"],
             "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
             "volumeMounts": [mount("github-config", "/agent"), mount("openbao-ca", "/run/openbao-ca"),
                 mount("openbao-identity", "/run/identity"), mount("github-token", "/run/github-openbao", False),
                 mount("openbao-scripts", "/scripts")],
             "startupProbe": {"exec": {"command": ["sh", "/scripts/check-github-token.sh"]},
                              "periodSeconds": 2, "timeoutSeconds": 15, "failureThreshold": 120}}
    spec = {"automountServiceAccountToken": False, "serviceAccountName": "github-connect",
            "securityContext": copy.deepcopy(POD_SECURITY), "initContainers": [agent],
            "containers": [{"name": "github-connect", "image": "aiforall-github-connect-service:latest", "imagePullPolicy": "Never",
                "env": [{"name": "RUN_ENV", "value": "development"}], "ports": [{"name": "https", "containerPort": 8443}],
                "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES), "volumeMounts": shared,
                "readinessProbe": {"tcpSocket": {"port": "https"}, "periodSeconds": 5},
                "startupProbe": {"tcpSocket": {"port": "https"}, "periodSeconds": 2, "failureThreshold": 30}}],
            "volumes": [identity("github-connect"), {"name": "github-config", "configMap": {"name": "github-openbao-config"}},
                {"name": "openbao-ca", "configMap": {"name": "openbao-public-ca"}},
                {"name": "connector-tls", "secret": {"secretName": "github-connect-server-tls", "defaultMode": 0o440}},
                {"name": "github-token", "emptyDir": {"medium": "Memory", "sizeLimit": "1Mi"}},
                {"name": "openbao-scripts", "configMap": {"name": script_name}}]}
    objects.append(resource("Deployment", "github-connect", spec={"replicas": 1,
        "selector": {"matchLabels": {"app.kubernetes.io/name": "github-connect"}},
        "template": {"metadata": {"labels": {"app.kubernetes.io/name": "github-connect"}}, "spec": spec}}))
    objects.append(resource("Service", "github-connect", spec={"selector": {"app.kubernetes.io/name": "github-connect"},
        "ports": [{"name": "https", "port": 8443, "targetPort": "https"}]}))
    objects.append(github_seed_job(run_id, script_name))
    objects.append(resource("NetworkPolicy", "allow-github-openbao", spec={
        "podSelector": {"matchExpressions": [{"key": "app.kubernetes.io/name", "operator": "In", "values": ["github-connect", "github-seed", "iam"]}]},
        "policyTypes": ["Egress"], "egress": [{"to": [{"podSelector": {"matchLabels": {"app.kubernetes.io/name": "openbao"}}}],
            "ports": [{"protocol": "TCP", "port": 8443}]}]}))
    for name, direction, source, target, port in [("allow-iam-github", "Egress", "iam", "github-connect", 8443),
        ("allow-github-iam", "Ingress", "github-connect", "iam", 8443),
        ("allow-github-jwks", "Egress", "github-connect", "iam", 8080),
        ("allow-iam-jwks", "Ingress", "iam", "github-connect", 8080)]:
        objects.append(resource("NetworkPolicy", name, spec={"podSelector": {"matchLabels": {"app.kubernetes.io/name": source}},
            "policyTypes": [direction], direction.lower(): [{"to" if direction == "Egress" else "from": [
                {"podSelector": {"matchLabels": {"app.kubernetes.io/name": target}}}], "ports": [{"protocol": "TCP", "port": port}]}]}))
