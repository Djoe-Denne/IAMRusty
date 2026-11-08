"""Explicit local OAuthConnect profile; no live reads, activation or secret values."""
import copy
import importlib.util
import json
from pathlib import Path
import re
import tomllib

# Like the renderer, support callers loading the hyphenated CLI by file path,
# without changing sys.path or activating a different shared project.
_spec = importlib.util.spec_from_file_location("oauth_openbao", Path(__file__).with_name("local_full_openbao.py"))
_bao = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_bao)
BAO, OVERLAY, POD_SECURITY, PYTHON, RESOURCES, SECURITY = (
    _bao.BAO, _bao.OVERLAY, _bao.POD_SECURITY, _bao.PYTHON, _bao.RESOURCES, _bao.SECURITY)
iam_template, identity, mount, resource, snapshot = (
    _bao.iam_template, _bao.identity, _bao.mount, _bao.resource, _bao.snapshot)

IMAGE = "aiforall-oauth-connect-service:latest"  # Replaced by the verified image receipt.
PUBLIC_ORIGIN = "https://127.0.0.1:18080"


def seed_inputs(github, simulated):
    """Validate both sources before any runtime operation; never log credentials."""
    try:
        for value in (github, simulated):
            if (set(value) != {"version", "client_id", "client_secret", "hmac_secret"}
                    or type(value["version"]) is not int or value["version"] != 1
                    or not isinstance(value["client_id"], str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", value["client_id"])
                    or not isinstance(value["client_secret"], str) or not 1 <= len(value["client_secret"]) <= 4096
                    or not value["client_secret"].strip() or any(ord(c) < 32 for c in value["client_secret"])
                    or not isinstance(value["hmac_secret"], str) or not re.fullmatch(r"[a-f0-9]{64}", value["hmac_secret"])):
                raise ValueError
        if (not re.fullmatch(r"[A-Za-z0-9_-]{8,128}", simulated["client_id"])
                or not 16 <= len(simulated["client_secret"]) <= 256
                or len({github["hmac_secret"], simulated["hmac_secret"], github["client_secret"], simulated["client_secret"]}) != 4):
            raise ValueError
    except (KeyError, TypeError, ValueError):
        raise ValueError("invalid or shared OAuthConnect credentials (values redacted)") from None
    return {"version": 1, "github": github, "simulated": simulated}


def seed_job(run_id, scripts):
    pod = {"serviceAccountName": "oauth-connect-seed", "automountServiceAccountToken": False,
           "restartPolicy": "Never", "securityContext": copy.deepcopy(POD_SECURITY),
           "containers": [{"name": "seed", "image": PYTHON,
               "command": ["python", "-B", "/scripts/openbao-workload.py", "oauth-connect-seed"],
               "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
               "volumeMounts": [mount("scripts", "/scripts"), mount("openbao-ca", "/run/openbao-ca"),
                               mount("openbao-identity", "/run/identity"), mount("private-inputs", "/inputs")]}],
           "volumes": [identity("oauth-connect-seed"), {"name": "scripts", "configMap": {"name": scripts}},
               {"name": "openbao-ca", "configMap": {"name": "openbao-public-ca"}},
               {"name": "private-inputs", "secret": {"secretName": "oauth-connect-seed-inputs-" + run_id,
                                                      "defaultMode": 0o440}}]}
    return resource("Job", "oauth-connect-seed", spec={"backoffLimit": 0, "activeDeadlineSeconds": 240,
        "ttlSecondsAfterFinished": 300, "template": {"metadata": {"labels": {
            "app.kubernetes.io/name": "oauth-connect-seed"}}, "spec": pod}})


def configure(objects, run_id, github_client_id, simulated_client_id):
    """All-or-nothing in-memory composition from the empty-registry base only."""
    planned = copy.deepcopy(objects)
    _configure(planned, run_id, github_client_id, simulated_client_id)
    objects[:] = planned


def _configure(objects, run_id, github_client_id, simulated_client_id):
    if (not re.fullmatch(r"[a-z0-9]{12,32}", run_id)
            or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", github_client_id)
            or not re.fullmatch(r"[A-Za-z0-9_-]{8,128}", simulated_client_id)):
        raise ValueError("invalid OAuthConnect public inputs")
    if any(o["metadata"]["name"] in {"oauth-connect", "github-connect", "gitlab-connect", "oauth-idp-simulator"}
           for o in objects):
        raise ValueError("OAuthConnect requires a distinct explicit profile")
    pod, volume, config, text = iam_template(objects)
    try:
        before = tomllib.loads(text)
        if config["kind"] != "ConfigMap" or before["idp"]["connectors"] != [] or re.search(r"__[A-Z0-9_]+__", text):
            raise ValueError
        if len(re.findall(r"(?m)^connectors = \[\]\s*$", text)) != 1:
            raise ValueError
        if (before["jwt"]["public_base_url"] != "http://127.0.0.1:8080"
                or before["jwt"]["issuer"] != "http://127.0.0.1:8080/iam"
                or before["auth"]["jwt"]["platform_issuer"] != "http://127.0.0.1:8080/iam"):
            raise ValueError
        rendered = re.sub(r"(?m)^connectors = \[\]\s*$", "", text, count=1)
        rendered = rendered.replace("http://127.0.0.1:8080", PUBLIC_ORIGIN)
        for slug, pkce in (("github", False), ("simulated", True)):
            connector = {"id": slug, "base_url": "https://oauth-connect:8443/oauth-connect/" + slug,
                         "hmac_secret": "__" + slug.upper() + "_OPENBAO_HMAC__", "pkce_supported": pkce,
                         "redirect_uris": [PUBLIC_ORIGIN + "/iam/api/auth/" + slug + "/" + path
                                           for path in ("callback", "relink-callback")]}
            rendered += "\n[[idp.connectors]]\n" + "\n".join(k + " = " + json.dumps(v) for k, v in connector.items()) + "\n"
        after = tomllib.loads(rendered)
        after["idp"]["connectors"] = []
        expected = copy.deepcopy(before)
        expected["jwt"]["public_base_url"] = PUBLIC_ORIGIN
        expected["jwt"]["issuer"] = PUBLIC_ORIGIN + "/iam"
        expected["auth"]["jwt"]["platform_issuer"] = PUBLIC_ORIGIN + "/iam"
        if after != expected:
            raise ValueError
    except (KeyError, TypeError, ValueError):
        raise ValueError("OAuthConnect requires an unambiguous empty IAM registry (values redacted)") from None
    config["data"]["development.toml"] = rendered
    # Align every existing consumer with the HTTPS public identity, without
    # altering its internal JWKS transport or introducing a second issuer.
    for obj in objects:
        name = obj["metadata"]["name"]
        if obj["kind"] == "Deployment" and name in {"hive", "telegraph", "manifesto", "lazaret", "sentinel-sync"}:
            prefix = "SENTINEL_SYNC_PRINCIPAL" if name == "sentinel-sync" else name.upper() + "_AUTH__JWT"
            env = obj["spec"]["template"]["spec"]["containers"][0].setdefault("env", [])
            for key, value in (("PLATFORM_ISSUER", PUBLIC_ORIGIN + "/iam"), ("ALLOW_HISTORICAL_PLATFORM_ISSUER", "false")):
                variable = prefix + "__" + key
                if any(e["name"] == variable for e in env):
                    raise ValueError("ambiguous OAuthConnect principal policy")
                env.append({"name": variable, "value": value})
    scripts = next(o for o in objects if o["kind"] == "ConfigMap"
                   and o["metadata"]["name"].startswith("local-full-bootstrap-scripts"))
    script_name = scripts["metadata"]["name"]
    scripts["data"]["check-oauth-token.sh"] = (OVERLAY / "helpers/check-github-token.sh").read_text().replace(
        "/run/github-openbao/token", "/run/oauth-openbao/token")
    for account in ("oauth-connect", "oauth-connect-seed", "oauth-simulator", "iam-openbao"):
        objects.append(resource("ServiceAccount", account, automountServiceAccountToken=False))
    snapshot(pod, "iam-oauth-hmac", "iam-openbao", script_name, volume["name"])
    app = next(c for c in pod["containers"] if c["name"] == "iam")
    if any(e["name"] == "SSL_CERT_FILE" for e in app.get("env", [])):
        raise ValueError("OAuthConnect requires the base IAM trust source")
    pod["volumes"].append({"name": "iam-trust", "emptyDir": {"medium": "Memory", "sizeLimit": "1Mi"}})
    pod["initContainers"].append({"name": "trusted-roots", "image": app["image"], "imagePullPolicy": "Never",
        "command": ["sh", "-ec", "cat /etc/ssl/certs/ca-certificates.crt /run/openbao-ca/ca.crt > /trust/ca-bundle.crt; chmod 0444 /trust/ca-bundle.crt"],
        "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
        "volumeMounts": [mount("openbao-ca", "/run/openbao-ca"), mount("iam-trust", "/trust", False)]})
    app.setdefault("env", []).append({"name": "SSL_CERT_FILE", "value": "/run/iam-trust/ca-bundle.crt"})
    app["volumeMounts"].append(mount("iam-trust", "/run/iam-trust"))
    public_config = (OVERLAY / "oauth-connect/connect.toml").read_text().replace(
        "__GITHUB_CLIENT_ID__", github_client_id).replace("__SIMULATED_CLIENT_ID__", simulated_client_id)
    objects.append(resource("ConfigMap", "oauth-connect-config", data={"development.toml": public_config,
        "agent.hcl": (OVERLAY / "oauth-connect/agent.hcl").read_text()}))
    agent = {"name": "openbao-agent", "image": BAO, "restartPolicy": "Always",
        "command": ["bao", "agent", "-config=/agent/agent.hcl", "-log-level=warn"],
        "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
        "volumeMounts": [mount("config", "/agent"), mount("openbao-ca", "/run/openbao-ca"),
            mount("openbao-identity", "/run/identity"), mount("oauth-token", "/run/oauth-openbao", False), mount("scripts", "/scripts")],
        "startupProbe": {"exec": {"command": ["sh", "/scripts/check-oauth-token.sh"]},
                         "periodSeconds": 2, "timeoutSeconds": 15, "failureThreshold": 120}}
    spec = {"serviceAccountName": "oauth-connect", "automountServiceAccountToken": False,
        "securityContext": copy.deepcopy(POD_SECURITY), "initContainers": [agent],
        "containers": [{"name": "oauth-connect", "image": IMAGE, "imagePullPolicy": "Never",
            "env": [{"name": "RUN_ENV", "value": "development"}], "ports": [{"name": "https", "containerPort": 8443}],
            "securityContext": copy.deepcopy(SECURITY), "resources": copy.deepcopy(RESOURCES),
            "volumeMounts": [mount("config", "/app/config"), mount("openbao-ca", "/run/openbao-ca"),
                            mount("connector-tls", "/run/connector-tls"), mount("oauth-token", "/run/oauth-openbao")],
            "readinessProbe": {"httpGet": {"scheme": "HTTPS", "path": "/oauth-connect/ready", "port": "https"}, "periodSeconds": 5},
            "startupProbe": {"tcpSocket": {"port": "https"}, "periodSeconds": 2, "failureThreshold": 30}}],
        "volumes": [identity("oauth-connect"), {"name": "config", "configMap": {"name": "oauth-connect-config"}},
            {"name": "openbao-ca", "configMap": {"name": "openbao-public-ca"}},
            {"name": "connector-tls", "secret": {"secretName": "oauth-connect-server-tls", "defaultMode": 0o440}},
            {"name": "oauth-token", "emptyDir": {"medium": "Memory", "sizeLimit": "1Mi"}},
            {"name": "scripts", "configMap": {"name": script_name}}]}
    objects.append(resource("Deployment", "oauth-connect", spec={"replicas": 1,
        "selector": {"matchLabels": {"app.kubernetes.io/name": "oauth-connect"}},
        "template": {"metadata": {"labels": {"app.kubernetes.io/name": "oauth-connect"}}, "spec": spec}}))
    objects.append(resource("Service", "oauth-connect", spec={"selector": {"app.kubernetes.io/name": "oauth-connect"},
        "ports": [{"name": "https", "port": 8443, "targetPort": "https"}]}))
    provider = {"client_id": simulated_client_id, "run_id": run_id, "instance": "simulated"}
    objects.append(resource("ConfigMap", "oauth-simulator-config", data={"provider.json": json.dumps(provider)}))
    objects.append(resource("ConfigMap", "oauth-connect-simulator", data={"oauth-idp-simulator.py": (
        OVERLAY.parents[3] / "tests/support/oauth-idp-simulator.py").read_text()}))
    sim = {"securityContext": copy.deepcopy(POD_SECURITY), "containers": [{"name": "provider", "image": PYTHON,
        "command": ["python", "-B", "/scripts/oauth-idp-simulator.py"],
        "env": [{"name": "OAUTH_PROOF_INPUT_FILE", "value": "/app/config/provider.json"}],
        "ports": [{"name": "http", "containerPort": 8080}], "securityContext": copy.deepcopy(SECURITY),
        "resources": copy.deepcopy(RESOURCES), "readinessProbe": {"httpGet": {"path": "/observations", "port": "http"}},
        "volumeMounts": [mount("config", "/app/config"), mount("simulator", "/scripts")]}],
        "volumes": [{"name": "config", "configMap": {"name": "oauth-simulator-config"}},
                    {"name": "simulator", "configMap": {"name": "oauth-connect-simulator"}}]}
    snapshot(sim, "oauth-simulator", "oauth-simulator", script_name)
    objects.append(resource("Deployment", "oauth-idp-simulator", spec={"replicas": 1,
        "selector": {"matchLabels": {"app.kubernetes.io/name": "oauth-idp-simulator"}},
        "template": {"metadata": {"labels": {"app.kubernetes.io/name": "oauth-idp-simulator"}}, "spec": sim}}))
    objects.append(resource("Service", "oauth-idp-simulator", spec={"selector": {"app.kubernetes.io/name": "oauth-idp-simulator"},
        "ports": [{"name": "http", "port": 8080, "targetPort": "http"}]}))
    objects.append(seed_job(run_id, script_name))
    # Default deny and namespace-wide DNS policy remain in effect. No IAM JWT/JWKS hop.
    for obj in objects:
        if obj["kind"] == "NetworkPolicy" and obj["metadata"]["name"] == "allow-openbao":
            obj["spec"]["ingress"][0]["from"][0]["podSelector"]["matchExpressions"][0]["values"] += [
                "oauth-connect", "oauth-connect-seed", "oauth-idp-simulator", "iam"]
    for source, target, port in (("iam", "oauth-connect", 8443), ("oauth-connect", "oauth-idp-simulator", 8080),
                                *((source, "openbao", 8443) for source in (
                                    "oauth-connect", "oauth-connect-seed", "oauth-idp-simulator", "iam"))):
        objects.append(resource("NetworkPolicy", "oauth-connect-" + source + "-to-" + target, spec={
            "podSelector": {"matchLabels": {"app.kubernetes.io/name": source}}, "policyTypes": ["Egress"],
            "egress": [{"to": [{"podSelector": {"matchLabels": {"app.kubernetes.io/name": target}}}],
                        "ports": [{"protocol": "TCP", "port": port}]}]}))
        if target != "openbao":
            objects.append(resource("NetworkPolicy", "oauth-connect-" + target + "-from-" + source, spec={
                "podSelector": {"matchLabels": {"app.kubernetes.io/name": target}}, "policyTypes": ["Ingress"],
                "ingress": [{"from": [{"podSelector": {"matchLabels": {"app.kubernetes.io/name": source}}}],
                             "ports": [{"protocol": "TCP", "port": port}]}]}))
    objects.append(resource("NetworkPolicy", "oauth-connect-vendor-https", spec={
        "podSelector": {"matchLabels": {"app.kubernetes.io/name": "oauth-connect"}}, "policyTypes": ["Egress"],
        "egress": [{"to": [{"ipBlock": {"cidr": "0.0.0.0/0", "except": ["10.0.0.0/8", "172.16.0.0/12",
            "192.168.0.0/16", "127.0.0.0/8", "169.254.0.0/16", "100.64.0.0/10", "224.0.0.0/4"]}}],
            "ports": [{"protocol": "TCP", "port": 443}]}]}))
