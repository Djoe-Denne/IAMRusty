"""Explicit lease-gated local-full validation; never deploy or seed.

Requires parent-provisioned exact leased probe pods and completed D operations.
No optional skips: a missing lease/resource/tool/fixture is a failure.
"""
import argparse
import hashlib
import http.client
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# Resolve D's shared authority from the fixed repository path, never PYTHONPATH.
anchor_spec = importlib.util.spec_from_file_location("local_full_target", ROOT / "ops/deploy/local_full_target.py")
anchor_module = importlib.util.module_from_spec(anchor_spec)
sys.modules["local_full_target"] = anchor_module
anchor_spec.loader.exec_module(anchor_module)
spec = importlib.util.spec_from_file_location("storage", ROOT / "ops/deploy/local-full-storage.py")
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)
CONTEXT = "kind-aiforall-local-full"
BASE_URL = "http://127.0.0.1:18080"
NAMESPACES = {"namespace": "aiforall-local-full", "apparatus_namespace": "aiforall-local-apparatus",
              "plugins_namespace": "aiforall-local-plugins"}
SMOKE_DEPLOYMENTS = ("postgres", "openfga", "redis", "localstack", "openbao", "mailhog",
                     "component-catalog", "zot", "iam", "hive", "manifesto", "telegraph",
                     "lazaret", "sentinel-sync", "ext-authz", "envoy-mesh")


class ValidationBlocked(ValueError):
    """An explicit prerequisite has no approved full setup; never a skip/PASS."""


def strict_json(path):
    def pairs(items):
        value = {}
        for key, item in items:
            if key in value:
                raise ValueError("duplicate JSON field")
            value[key] = item
        return value
    return json.loads(path.read_text(encoding="utf-8-sig"), object_pairs_hook=pairs,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON value")))


def read_environment(path):
    value = strict_json(path)
    required = {"version", "profile", "context", "cluster", "lease_path", "base_url", *NAMESPACES}
    if not isinstance(value, dict) or not required <= value.keys() or value.keys() - required - {"public_ca_path"}:
        raise ValueError("environment fields must match the v1 schema exactly")
    if type(value["version"]) is not int or value["version"] != 1 or value["profile"] != "local-full":
        raise ValueError("environment v1 local-full profile required")
    if value["context"] != CONTEXT or value["cluster"] != "aiforall-local-full":
        raise ValueError("foreign or contradictory environment target")
    if value["base_url"] != BASE_URL or any(value[k] != v for k, v in NAMESPACES.items()):
        raise ValueError("foreign or contradictory URL/namespaces")
    for key in ("lease_path", "public_ca_path"):
        if key in value and (not isinstance(value[key], str) or not Path(value[key]).is_absolute()):
            raise ValueError("environment paths must be explicit and absolute")
    return value


def bind_environment(args):
    if args.environment:
        if any(getattr(args, key) is not None for key in ("lease", "namespace", "context")):
            raise ValueError("environment cannot be combined with lease/namespace/context overrides")
        args.environment_data = read_environment(args.environment)
        args.lease = Path(args.environment_data["lease_path"])
        args.namespace = args.environment_data["namespace"]
        args.context = args.environment_data["context"]
    else:
        args.environment_data = None
        if args.lease is None or args.namespace is None:
            raise ValueError("explicit environment or legacy lease/namespace required")
        args.context = args.context or CONTEXT
    if args.action in {"smoke", "e2e", "prepare-probes", "cleanup-probes"} and not args.environment_data:
        raise ValueError("this mode requires an explicit environment")


def smoke(args, data):
    # Only guarded GET/wait. Readiness is not business/AuthZ proof.
    for namespace in NAMESPACES.values():
        namespace_owned(data, namespace)
        args.executed += 1
    for namespace, name in [(args.namespace, name) for name in SMOKE_DEPLOYMENTS] + [
            (NAMESPACES["apparatus_namespace"], "apparatus-operator")]:
        deployment = owned(data, "Deployment", name, namespace)
        runtime.kubectl("wait", "--for=condition=Available", "deployment/" + name,
                        "-n", namespace, "--timeout=120s")
        current = obj("Deployment", name, namespace)
        if current["metadata"]["uid"] != deployment["metadata"]["uid"]:
            raise AssertionError("deployment replaced during smoke")
        desired = current["spec"].get("replicas", 1)
        status = current.get("status", {})
        if desired < 1 or status.get("observedGeneration", 0) < current["metadata"].get("generation", 1) or any(
                status.get(key, 0) < desired for key in ("availableReplicas", "readyReplicas", "updatedReplicas")):
            raise AssertionError("required deployment is not fully available")
        args.executed += 1
    claims = json.loads(runtime.kubectl("get", "pvc", "-n", args.namespace, "-o", "json"))["items"]
    if not claims or any(p.get("status", {}).get("phase") != "Bound" for p in claims):
        raise AssertionError("required persistent claims are not Bound")
    args.executed += 1


def public_cases(path):
    cases = strict_json(path)
    if not isinstance(cases, list) or not cases:
        raise ValueError("nonempty explicit route cases required")
    ids = set()
    for case in cases:
        if not isinstance(case, dict) or set(case) - {"id", "method", "path", "statuses", "json"}:
            raise ValueError("unknown route case field")
        identifier = case.get("id", "")
        path = case.get("path", "")
        if not isinstance(identifier, str) or not re.fullmatch(r"[a-z0-9-]{1,80}", identifier) or identifier in ids:
            raise ValueError("unique nonsecret route case IDs required")
        ids.add(identifier)
        if not isinstance(case.get("method"), str) or case["method"] not in {"GET", "HEAD", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"} or not isinstance(path, str) or not path.startswith("/iam/") or any(
                ord(c) <= 32 or ord(c) >= 127 for c in path) or "#" in path or "\\" in path:
            raise ValueError("exact local IAM route required")
        statuses = case.get("statuses")
        if not isinstance(statuses, list) or not statuses or any(type(s) is not int or not 100 <= s <= 599 for s in statuses):
            raise ValueError("explicit route statuses required")
        if "json" in case and case["json"] != {}:
            raise ValueError("public route matrix accepts empty validation bodies only")
    return cases


def public_route_matrix(args, data):
    namespace_owned(data, args.namespace)
    args.oauth_login_unavailable = 0
    # Direct fixed loopback connection: no ambient proxies, redirects or URL joins.
    # The authority rechecks endpoint/mapping/UID before EACH HTTP request.
    for case in args.route_cases:
        runtime.target().check()
        connection = http.client.HTTPConnection("127.0.0.1", 18080, timeout=20)
        try:
            connection.request(case["method"], case["path"], body=b"{}" if "json" in case else None,
                               headers={"Content-Type": "application/json"} if "json" in case else {})
            response = connection.getresponse()
            status = response.status
            # Do not print or persist bodies, cookies, headers, or credentials.
            if status not in case["statuses"]:
                raise AssertionError("route matrix status mismatch (response redacted)")
            if status == 422 and re.fullmatch(r"/iam/api/auth/[^/]+/login", case["path"]):
                args.oauth_login_unavailable += 1
            args.executed += 1
        finally:
            connection.close()


def public_coverage_report(args, status, evidence):
    if args.action != "route-matrix" or args.environment is None:
        return {}
    return {"coverage": {
        "public": {"status": status, "evidence": evidence},
        "authenticated": {"status": "NOT RUN", "setup": "BLOCKED", "channel": "e2e --suite --inputs",
                          "reason": "approved full authenticated setup unavailable"}},
        "oauth_login_unavailable": getattr(args, "oauth_login_unavailable", 0)}


def explicit_probe_action(args, data):
    # No approved full setup exists yet. Never reuse mutating legacy drivers.
    inputs = strict_json(args.inputs) if args.inputs else None
    if args.action == "cleanup-probes":
        if inputs is not None and (not isinstance(inputs, dict) or set(inputs) != {"version", "created_resources"}
                                   or type(inputs["version"]) is not int or inputs["version"] != 1
                                   or inputs["created_resources"] != []):
            raise ValidationBlocked("cleanup refused: no resources were created or attested by prepare-probes")
        args.executed += 1
        return  # Explicit no-op: prepare-probes has never created resources.
    if not isinstance(inputs, dict) or type(inputs.get("version")) is not int or inputs["version"] != 1:
        raise ValidationBlocked("private v1 inputs with declared operator state required")
    raise ValidationBlocked("approved full setup/UID-attested probes unavailable; legacy seed/rollout forbidden")


def git_revision():
    directory = ROOT / ".git"
    if directory.is_file():
        directory = (ROOT / directory.read_text().strip().removeprefix("gitdir: ")).resolve()
    head = (directory / "HEAD").read_text().strip()
    if head.startswith("ref: "):
        reference = head[5:]
        common = (directory / (directory / "commondir").read_text().strip()).resolve() if (directory / "commondir").exists() else directory
        ref = common / reference
        if ref.exists():
            head = ref.read_text().strip()
        else:
            head = next((line.split()[0] for line in (common / "packed-refs").read_text().splitlines()
                         if line.endswith(" " + reference)), "")
    if not re.fullmatch(r"[a-f0-9]{40}", head):
        raise ValueError("Git source revision unavailable")
    return head


def obj(kind, name, namespace=None):
    args = ["get", kind, name, "-o", "json"]
    if namespace:
        args += ["-n", namespace]
    return json.loads(runtime.kubectl(*args))


def owned(data, kind, name, namespace=None):
    actual = obj(kind, name, namespace)
    metadata = actual["metadata"]
    if not any(r.get("kind", "").lower() == kind.lower() and r.get("name") == name
               and r.get("namespace") == namespace and r.get("uid") == metadata["uid"]
               for r in data.get("ownedResources", [])):
        raise ValueError("exact resource UID not in parent lease")
    return actual


def namespace_owned(data, name):
    if not name.startswith("aiforall-local-"):
        raise ValueError("local-full namespaces only")
    if obj("namespace", name)["metadata"]["uid"] != data.get("namespaceUids", {}).get(name):
        raise ValueError("namespace UID not leased")


def guarded_curl(namespace, pod, arguments, timeout):
    # Anchor.kubectl checks CP publication/fixed-file identity BEFORE UID and
    # every exec. Report curl's remote status without bypassing that authority
    # merely to preserve NP's deliberate curl exit 28; the wrapper exits zero.
    script = 'out=$(curl "$@"); rc=$?; printf "%s %s\\n" "$rc" "$out"'
    output = runtime.kubectl("exec", "-n", namespace, pod, "--", "sh", "-c", script,
                             "guarded-curl", *arguments, timeout=timeout)
    match = re.fullmatch(r"([0-9]{1,3}) ([0-9]{3})", output)
    if not match:
        raise AssertionError("invalid guarded curl result; response/credentials redacted")
    return int(match[1]), match[2]


def probe(args, data):
    namespace_owned(data, args.namespace)
    service = owned(data, "Service", args.service, args.namespace)
    ip = service["spec"]["clusterIP"]
    if ip in {None, "None", ""}:
        raise ValueError("probe requires an exact service ClusterIP")
    dns_host = f"{args.service}.{args.namespace}.svc.cluster.local"
    host = args.tls_host or dns_host
    results = []
    for probe_namespace, pod in [(args.allowed_namespace, args.allowed_pod), (args.denied_namespace, args.denied_pod)]:
        namespace_owned(data, probe_namespace)
        resource = owned(data, "Pod", pod, probe_namespace)
        if resource.get("status", {}).get("phase") != "Running":
            raise ValueError("probe pod not Running")
        # Assert DNS separately. The TCP test uses the same exact ClusterIP on
        # both sides, preventing DNS failure/wrong route from masquerading as NP.
        runtime.kubectl("exec", "-n", probe_namespace, pod, "--", "nslookup", dns_host)
        command = ["--silent", "--show-error", "--output", "/dev/null", "--write-out", "%{http_code}",
                   "--connect-timeout", "5", "--max-time", "10", "--resolve", f"{host}:{args.port}:{ip}",
                   "--cacert", args.ca_path, "--cert", args.cert_path, "--key", args.key_path,
                   f"https://{host}:{args.port}{args.path}"]
        results.append(guarded_curl(probe_namespace, pod, command, timeout=20))
    if results[0][0] != 0 or not re.fullmatch(r"[1-5][0-9]{2}", results[0][1]):
        raise AssertionError("authorized source did not reach the exact HTTP route (TLS/DNS/connectivity not proven)")
    if results[1] != (28, "000"):
        raise AssertionError("forbidden source must time out; refusal/DNS/TLS errors are not NP-denial proof")
    print("PASS T6 exact destination: authorized HTTP; forbidden TCP timeout; DNS works for both")


def route_matrix(args, data):
    namespace_owned(data, args.namespace)
    namespace_owned(data, args.allowed_namespace)
    service = owned(data, "Service", args.service, args.namespace)
    pod = owned(data, "Pod", args.allowed_pod, args.allowed_namespace)
    if pod.get("status", {}).get("phase") != "Running":
        raise ValueError("route probe must be Running")
    ip = service["spec"]["clusterIP"]
    if not ip or ip == "None":
        raise ValueError("exact Envoy ClusterIP required")
    dns_host = f"{args.service}.{args.namespace}.svc.cluster.local"
    host = args.tls_host or dns_host
    runtime.kubectl("exec", "-n", args.allowed_namespace, args.allowed_pod, "--", "nslookup", dns_host)
    cases = json.loads(args.cases.read_text())
    if not isinstance(cases, list) or not cases:
        raise ValueError("nonempty explicit route cases required")
    ids = set()
    for case in cases:
        identifier = case.get("id", "")
        if not re.fullmatch(r"[a-z0-9-]{1,80}", identifier) or identifier in ids:
            raise ValueError("unique nonsecret case IDs required")
        ids.add(identifier)
        method = case.get("method")
        path = case.get("path", "")
        statuses = case.get("statuses")
        if method not in {"GET", "HEAD", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"} or not path.startswith("/iam/") or "\n" in path:
            raise ValueError("invalid exact IAM route case")
        if not isinstance(statuses, list) or not statuses or any(not isinstance(s, int) or not 100 <= s <= 599 for s in statuses):
            raise ValueError("explicit expected statuses required")
        # No Authorization/principal headers. mTLS authenticates only this hop.
        command = ["--silent", "--show-error", "--path-as-is", "--output", "/dev/null", "--write-out", "%{http_code}",
                   "--connect-timeout", "5", "--max-time", "15", "--resolve", f"{host}:{args.port}:{ip}",
                   "--cacert", args.ca_path, "--cert", args.cert_path, "--key", args.key_path]
        command += ["--head"] if method == "HEAD" else ["--request", method]
        if "json" in case:
            if case["json"] != {}:
                raise ValueError("route matrix accepts empty validation bodies only, never credentials")
            command += ["--header", "content-type: application/json", "--data", "{}"]
        command += [f"https://{host}:{args.port}{path}"]
        returncode, status = guarded_curl(args.allowed_namespace, args.allowed_pod, command, timeout=25)
        if returncode or status not in {str(s) for s in statuses}:
            raise AssertionError("route matrix failed: " + identifier + " (body/credentials not logged)")
        print("PASS route case: " + identifier)


def domain_digest(args, data, namespace, database, pod):
    namespace_owned(data, namespace)
    owned(data, "Pod", pod, namespace)
    query = args.invariant_sql.read_text(encoding="utf-8").strip()
    if not query.upper().startswith("SELECT ") or ";" in query:
        raise ValueError("one ordered domain SELECT required; no fixture mutation")
    # READ ONLY prevents side effects even from a function inside a SELECT.
    result = runtime.kubectl("exec", "-i", "-n", namespace, pod, "--", "psql", "-X", "-qAt",
                             "-U", "postgres", "-d", database, "-v", "ON_ERROR_STOP=1",
                             stdin="BEGIN READ ONLY;\n" + query + ";\nROLLBACK;\n")
    if not result:
        raise AssertionError("domain invariant fixture must contain real persistent data")
    return {"query_sha256": hashlib.sha256(query.encode()).hexdigest(),
            "rows_sha256": hashlib.sha256(result.encode()).hexdigest()}


def snapshot(args, data):
    namespace_owned(data, args.namespace)
    deployment = owned(data, "Deployment", args.deployment, args.namespace)
    pods = json.loads(runtime.kubectl("get", "pods", "-n", args.namespace, "-l",
                      "app.kubernetes.io/name=" + args.deployment, "-o", "json"))["items"]
    for pod in pods:
        references = [r for r in pod["metadata"].get("ownerReferences", [])
                      if r.get("controller") and r.get("kind") == "ReplicaSet"]
        if len(references) != 1:
            raise AssertionError("selector match alone is not deployment ownership")
        replica = obj("ReplicaSet", references[0]["name"], args.namespace)
        if replica["metadata"]["uid"] != references[0]["uid"] or not any(
                r.get("controller") and r.get("kind") == "Deployment" and r.get("uid") == deployment["metadata"]["uid"]
                for r in replica["metadata"].get("ownerReferences", [])):
            raise AssertionError("pod must be a controlled child of the exact leased deployment")
    template = deployment["spec"]["template"]
    checksum = template.get("metadata", {}).get("annotations", {}).get("aiforall.dev/config-sha256", "")
    if not re.fullmatch(r"[a-f0-9]{64}", checksum):
        raise AssertionError("D's full configuration checksum is absent")
    images = [c["image"] for c in template["spec"]["containers"]]
    if any(not re.search(r"(?::sha256-|@sha256:)[a-f0-9]{64}$", image) for image in images):
        raise AssertionError("representative deployment must use exact versioned image identity")
    if not pods or any(not any(c.get("type") == "Ready" and c.get("status") == "True"
                              for c in p.get("status", {}).get("conditions", [])) for p in pods):
        raise AssertionError("all representative replicas must be Ready")
    claims = json.loads(runtime.kubectl("get", "pvc", "-n", args.namespace, "-o", "json"))["items"]
    if not claims or any(p.get("status", {}).get("phase") != "Bound" for p in claims):
        raise AssertionError("persistent source PVCs must be Bound")
    return {"namespace": args.namespace, "deployment_uid": deployment["metadata"]["uid"],
            "template_sha256": hashlib.sha256(json.dumps(template, sort_keys=True).encode()).hexdigest(),
            "configuration_sha256": checksum,
            "images": images,
            "pod_uids": sorted(p["metadata"]["uid"] for p in pods),
            "pod_nodes": sorted({p["spec"]["nodeName"] for p in pods}),
            "pvcs": sorted([p["metadata"]["uid"], p["spec"]["volumeName"]] for p in claims),
            "domain": domain_digest(args, data, args.namespace, args.database, args.database_pod)}


def run(args):
    if args.context != CONTEXT:
        raise ValueError("full-only target required; no legacy lease adoption")
    data = runtime.lease(args.lease)
    if args.action == "network-policy":
        probe(args, data)
        return
    if args.action == "route-matrix":
        route_matrix(args, data)
        return
    before = json.loads(args.before.read_text()) if args.before else None
    if args.action == "restore-check":
        if not before or args.namespace == before["namespace"] or not args.namespace.startswith("aiforall-local-restore-"):
            raise ValueError("distinct D-created restore target and source snapshot required")
        current = domain_digest(args, data, args.namespace, args.database, args.database_pod)
        if current != before["domain"]:
            raise AssertionError("restored ordered domain invariants differ")
        claims = json.loads(runtime.kubectl("get", "pvc", "-n", args.namespace, "-o", "json"))["items"]
        source = {p[1] for p in before["pvcs"]}
        if not claims or any(p.get("status", {}).get("phase") != "Bound" or not p["spec"].get("volumeName") or p["spec"]["volumeName"] in source for p in claims):
            raise AssertionError("restore must use distinct bound storage")
        print("PASS T7 distinct restore storage and exact ordered domain data")
        return
    current = snapshot(args, data)
    if args.action == "snapshot":
        with args.output.open("x", encoding="utf-8") as target:
            json.dump(current, target, indent=2)
        print("snapshot persisted (metadata/digests only; not acceptance proof)")
        return
    if not before or before["deployment_uid"] != current["deployment_uid"] or before["namespace"] != current["namespace"]:
        raise AssertionError("same leased deployment required; no replacement/adoption")
    if before["domain"] != current["domain"] or before["pvcs"] != current["pvcs"]:
        raise AssertionError("domain data or persistent storage identity changed")
    if args.action == "rollout-check":
        if set(before["pod_uids"]) & set(current["pod_uids"]):
            raise AssertionError("all old replicas must have been replaced by rollout")
        if before["template_sha256"] == current["template_sha256"] or before["configuration_sha256"] == current["configuration_sha256"] or before["images"] == current["images"]:
            raise AssertionError("versioned image AND configuration/template must change")
    else:
        if before["pod_uids"] == current["pod_uids"]:
            raise AssertionError("no actual pod rescheduling/replacement observed")
        if data.get("nodeFailures"):
            raise AssertionError("owned interrupted worker still requires recovery")
        if len(current["pod_nodes"]) < 2:
            raise AssertionError("representative replicas must span surviving/recovered workers")
    print("PASS T7 replacement, persistent domain/storage invariants and " + args.action)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["smoke", "e2e", "prepare-probes", "cleanup-probes", "network-policy", "route-matrix", "snapshot", "rollout-check", "recovery-check", "restore-check"])
    parser.add_argument("--environment", type=Path)
    parser.add_argument("--suite", choices=["mesh-authn", "apparatus"])
    parser.add_argument("--inputs", type=Path)
    parser.add_argument("--context", choices=[CONTEXT])
    parser.add_argument("--lease", type=Path)
    parser.add_argument("--namespace")
    parser.add_argument("--deployment")
    parser.add_argument("--database")
    parser.add_argument("--database-pod")
    parser.add_argument("--invariant-sql", type=Path)
    parser.add_argument("--before", type=Path)
    parser.add_argument("--output", type=Path)
    for option in ["allowed-namespace", "allowed-pod", "denied-namespace", "denied-pod", "service", "ca-path", "cert-path", "key-path"]:
        parser.add_argument("--" + option)
    parser.add_argument("--port", type=int)
    parser.add_argument("--path")
    parser.add_argument("--tls-host")
    parser.add_argument("--cases", type=Path)
    args = parser.parse_args()
    args.executed = 0
    expected = 1
    revision = "NOT RUN"
    try:
        revision = git_revision()
        bind_environment(args)
        modern = args.action in {"smoke", "e2e", "prepare-probes", "cleanup-probes"} or (
            args.action == "route-matrix" and args.environment_data is not None)
        if modern:
            legacy_options = ("allowed_namespace", "allowed_pod", "denied_namespace", "denied_pod", "service",
                              "ca_path", "cert_path", "key_path", "tls_host", "port", "path", "deployment",
                              "database", "database_pod", "invariant_sql", "before", "output")
            if any(getattr(args, key) is not None for key in legacy_options):
                raise ValueError("environment mode rejects legacy fixture/transport overrides")
            if (args.cases and args.action != "route-matrix") or (args.suite and args.action != "e2e") or (
                    args.inputs and args.action not in {"e2e", "prepare-probes", "cleanup-probes"}):
                raise ValueError("ambiguous action-specific inputs")
            if args.action == "route-matrix":
                if not args.cases:
                    raise ValidationBlocked("explicit route cases required")
                args.route_cases = public_cases(args.cases)
                expected = len(args.route_cases)
            elif args.action == "smoke":
                expected = len(NAMESPACES) + len(SMOKE_DEPLOYMENTS) + 2
            elif args.action == "e2e" and (not args.suite or not args.inputs):
                raise ValidationBlocked("explicit suite and private inputs required")
            with runtime.operation_lock(args.lease, "acceptance-" + args.action):
                data = runtime.lease(args.lease)
                if args.action == "smoke":
                    smoke(args, data)
                elif args.action == "route-matrix":
                    public_route_matrix(args, data)
                else:
                    explicit_probe_action(args, data)
        else:
            args.port = 10000 if args.port is None else args.port
            args.path = "/iam/api/me" if args.path is None else args.path
            required = (["allowed_namespace", "allowed_pod", "denied_namespace", "denied_pod", "service", "ca_path", "cert_path", "key_path"]
                        if args.action == "network-policy" else ["allowed_namespace", "allowed_pod", "service", "ca_path", "cert_path", "key_path", "cases"]
                        if args.action == "route-matrix" else ["database", "database_pod", "invariant_sql"])
            if args.action not in {"network-policy", "route-matrix", "restore-check"}:
                required += ["deployment"]
            required += ["output"] if args.action == "snapshot" else ([] if args.action in {"network-policy", "route-matrix"} else ["before"])
            if any(getattr(args, name) is None for name in required):
                raise ValidationBlocked("missing required action-specific fixture/evidence argument")
            if not 1 <= args.port <= 65535 or not args.path.startswith("/"):
                raise ValueError("invalid exact target")
            if args.tls_host and not re.fullmatch(r"[a-z0-9.-]+", args.tls_host):
                raise ValueError("invalid explicit TLS hostname")
            with runtime.operation_lock(args.lease, "acceptance-" + args.action):
                run(args)
                args.executed = 1
    except Exception as error:
        blocked = isinstance(error, (ValidationBlocked, ValueError, OSError, anchor_module.AnchorError))
        executed = args.executed if blocked else min(expected, args.executed + 1)
        print(json.dumps({"git": revision, "target": CONTEXT, "mode": args.action,
                          **public_coverage_report(args, "BLOCKED" if blocked else "FAIL", "NOT RUN" if not executed else "LIVE"),
                          "status": "BLOCKED" if blocked else "FAIL", "evidence": "NOT RUN" if not executed else "LIVE",
                          "expected": expected, "executed": executed, "failed": 0 if blocked else 1,
                          "blocked": max(0, expected - executed),
                          "reason": str(error) if isinstance(error, ValidationBlocked) else
                                    "validation rejected or failed (" + type(error).__name__ + "); response/credentials redacted"}))
        return 2 if blocked else 1
    print(json.dumps({"git": revision, "target": CONTEXT, "mode": args.action, "status": "PASS",
                      **public_coverage_report(args, "PASS", "LIVE"),
                      "evidence": "NOT RUN" if args.action == "cleanup-probes" else "LIVE",
                      "expected": expected, "executed": args.executed, "failed": 0, "blocked": 0,
                      "scope": "readiness only; business/AuthZ NOT RUN" if args.action == "smoke" else
                               "public routes only; authenticated business/AuthZ NOT RUN (e2e setup BLOCKED)" if args.action == "route-matrix" and args.environment_data else
                               "no resources created; cleanup no-op" if args.action == "cleanup-probes" else
                               "LocalStack/MailHog/catalogue/OpenBao DEV/Zot are simulated dependencies; no production equivalence"}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
