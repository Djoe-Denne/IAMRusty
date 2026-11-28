"""Run-owned backing databases and deployment for the real monolith binary."""

import base64
import hmac
import json
import re
import secrets
import time
import uuid

from binaries import RECOVERY_ROLE, Refused
import inputs
from runtime import NS

PORT = 18449
MIGRATIONS = {
    "hive": "hivemigration",
    "manifesto": "manifestomigration",
    "telegraph": "telegraphmigration",
    "lazaret": "migration",
}
STARTUP_FAILURES = {
    "CONFIG_SECTION_MISSING": rb"missing field",
    "PRINCIPAL_POLICY": rb"PRINCIPAL_POLICY_UNAVAILABLE|PRINCIPAL_INVALID",
    "IAM_SIGNER": rb"IAM organization signer",
    "DATABASE": rb"failed to connect to database|invalid database|pool timed out",
    "TEMPLATE": rb"Failed to create template service",
    "BINDING_GRANT": rb"binding grant snapshot reader unavailable|Invalid grant snapshot auth configuration",
    "IDENTITY": rb"Invalid identity configuration|Invalid TLS material",
}


def create_monolith(campaign):
    values = campaign.state["values"]
    name = campaign.state["name"] + "-monolith"
    database = values["database"]
    postgres = f"postgres.{NS}.svc.cluster.local"
    loopback_iam = "http://127.0.0.1:8080/iam"
    secret = {"snapshot-key": secrets.token_hex(32)}

    def from_secret(key):
        return {"secretKeyRef": {"name": name, "key": key}}

    configs, _, _ = inputs.configuration(values)
    config = configs["iam"].replace('tls_enabled = true', 'tls_enabled = false', 1)
    for url in (f"https://iam.{NS}.svc.cluster.local:8443/iam", "https://127.0.0.1:18443/iam"):
        config = config.replace(f'jwks_url = "{url}/.well-known/jwks.json"',
                                f'jwks_url = "{loopback_iam}/.well-known/jwks.json"')
    config = f'internal_service_token = "{campaign.session_replica.liveness_token}"\n' + config
    config += f"""
[auth.session_liveness]
endpoint = "{loopback_iam}/internal/auth/session-liveness"
token_env = "K6_MONOLITH_LIVENESS"
timeout_ms = 1000
[openfga]
scheme = "https"
host = "openfga.{NS}.svc.cluster.local"
port = 8080
[iam_service]
base_url = "http://127.0.0.1:8080"
api_key = ""
timeout_seconds = 10
[external_provider_service]
base_url = "http://127.0.0.1:8080"
timeout_seconds = 10
max_retries = 0
"""
    pki = campaign.root / "private/pki"
    secret.update({"k6.toml": config, "platform-signing.pem": (pki / "platform-signing.pem").read_bytes(),
                   "platform-signing.pub": (pki / "platform-signing.pub").read_bytes(),
                   "K6_MONOLITH_LIVENESS": campaign.session_replica.liveness_token})
    image = campaign.state["images"]["roles"][RECOVERY_ROLE]["tag"]
    campaign.r.cli("kind", ["load", "docker-image", "--name", campaign.state["cluster"], image], timeout=240)

    mounts = [{"name": "tls", "mountPath": "/app/certs", "readOnly": True}]
    postgres_env = [{"name": "PGSSLMODE", "value": "verify-full"},
                    {"name": "PGSSLROOTCERT", "value": "/app/certs/ca.crt"},
                    {"name": "SSL_CERT_FILE", "value": "/app/certs/ca-bundle.crt"}]
    init = []
    env = [*postgres_env, {"name": "RUN_ENV", "value": "k6"},
           {"name": "K6_MONOLITH_LIVENESS", "valueFrom": from_secret("K6_MONOLITH_LIVENESS")}]
    for service, executable in MIGRATIONS.items():
        service_db = database + "_" + service
        campaign.sql(f'CREATE DATABASE "{service_db}";')
        secret[service + "-database-url"] = (
            f"postgresql://postgres:{values['database_password']}@{postgres}:5432/{service_db}"
            "?sslmode=verify-full&sslrootcert=/app/certs/ca.crt"
        )
        init.append(container(service, image, ["/app/bin/" + executable, "up"],
                              [*postgres_env, {"name": "DATABASE_URL", "valueFrom": from_secret(service + "-database-url")}],
                              mounts))
        prefix = service.upper()
        env.extend([
            {"name": prefix + "_DATABASE__DB", "value": service_db},
            {"name": prefix + "_QUEUE__ENABLED", "value": "false"},
        ])
    for service in ("IAM", "HIVE", "TELEGRAPH", "MANIFESTO"):
        for field, key in (("STORE_ID", "store-id"), ("AUTHORIZATION_MODEL_ID", "model-id")):
            env.append({"name": f"{service}_OPENFGA__{field}", "valueFrom": {
                "configMapKeyRef": {"name": "local-full-fga", "key": key}}})
    env.extend([
        {"name": "TELEGRAPH_COMMUNICATION__EMAIL__PROVIDER", "value": "dummy"},
        {"name": "TELEGRAPH_COMMUNICATION__TEMPLATE__TEMPLATE_DIR", "value": "/tmp/k6-telegraph-templates"},
        {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__HS256_SECRET", "valueFrom": from_secret("snapshot-key")},
        {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__ISSUER", "value": "aiforall-platform"},
        {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__AUDIENCE", "value": "manifesto-bindings"},
        {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__SERVICE_PRINCIPAL_ISSUER", "value": "aiforall-platform"},
    ])
    main = container("monolith", image, ["/app/bin/oodhive-monolith"], env,
                     [*mounts, {"name": "config", "mountPath": "/app/config", "readOnly": True}])
    main["readinessProbe"] = {"httpGet": {"path": "/iam/health", "port": 8080},
                              "periodSeconds": 2, "timeoutSeconds": 2}
    main["ports"] = [{"name": "http", "containerPort": 8080}]
    main["resources"]["limits"]["memory"] = "1536Mi"
    campaign.secret(name, secret)
    labels = {"aiforall.dev/k6-run": campaign.state["run_id"], "aiforall.dev/k6-role": "monolith"}
    campaign.create({
        "apiVersion": "apps/v1", "kind": "Deployment", "metadata": {"name": name},
        "spec": {"replicas": 1, "selector": {"matchLabels": labels}, "template": {
            "metadata": {"labels": labels | {"aiforall.dev/kind-minimal": "true"}},
            "spec": {"automountServiceAccountToken": False, "securityContext": {
                "runAsNonRoot": True, "runAsUser": 1000, "runAsGroup": 1000, "fsGroup": 1000,
                "seccompProfile": {"type": "RuntimeDefault"}},
                "initContainers": init, "containers": [main], "volumes": [
                    {"name": "tls", "secret": {"secretName": campaign.state["name"] + "-iam-tls", "defaultMode": 0o440,
                     "items": [{"key": key, "path": key} for key in ("ca.crt", "ca-bundle.crt")]}},
                    {"name": "config", "secret": {"secretName": name, "defaultMode": 0o440,
                     "items": [{"key": key, "path": key} for key in ("k6.toml", "platform-signing.pem",
                                                                   "platform-signing.pub")]}},
                ]},
        }},
    })
    campaign.rollout(name)
    campaign.forward("monolith", PORT, 8080)
    return secret["snapshot-key"]


def monolith_startup_categories(campaign):
    try:
        pod = campaign.pod("monolith")
        raw = campaign.r.k(["logs", pod["metadata"]["name"], "--container=monolith",
                            "--previous", "--tail=60", "--limit-bytes=16384"], timeout=10)
    except Refused:
        return ["LOG_UNAVAILABLE"]
    return classify_startup(raw)


def classify_startup(raw):
    return [name for name, pattern in STARTUP_FAILURES.items() if re.search(pattern, raw, re.I)] or ["UNCLASSIFIED"]


def snapshot_fixture(sql, database, subject, key):
    subject = str(uuid.UUID(subject))
    project, component = str(uuid.uuid4()), str(uuid.uuid4())
    sql(f"""
        BEGIN;
        INSERT INTO public.projects (id, name, owner_type, owner_id, created_by)
        VALUES ('{project}', 'recovery workload fixture', 'personal', '{subject}', '{subject}');
        INSERT INTO public.project_components (id, project_id, component_type)
        VALUES ('{component}', '{project}', 'taskboard');
        INSERT INTO public.apparatus_bindings (component_id, source) VALUES ('{component}', 'legacy');
        COMMIT;
    """, database)

    def segment(value):
        return base64.urlsafe_b64encode(json.dumps(value).encode()).rstrip(b"=")

    now = int(time.time())
    signing_input = segment({"alg": "HS256", "typ": "JWT"}) + b"." + segment({
        "iss": "aiforall-platform", "sub": str(uuid.uuid4()), "aud": "manifesto-bindings",
        "iat": now, "exp": now + 300, "jti": str(uuid.uuid4()),
    })
    digest = hmac.digest(key.encode(), signing_input, "sha256")
    token = (signing_input + b"." + base64.urlsafe_b64encode(digest).rstrip(b"=")).decode()
    return f"/api/projects/{project}/bindings/{component}", project, component, token


def container(name, image, command, environment, mounts):
    return {"name": name, "image": image, "imagePullPolicy": "Never", "command": command,
            "env": environment, "volumeMounts": mounts,
            "securityContext": {"allowPrivilegeEscalation": False, "capabilities": {"drop": ["ALL"]}},
            "resources": {"requests": {"cpu": "50m", "memory": "64Mi"},
                          "limits": {"cpu": "2", "memory": "512Mi"}}}
