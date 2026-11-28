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
from pki import COMPANIONS
from runtime import NS
from session_replica import IAM_URL

PORT = 18449
STANDALONE_PORTS = {"hive": 18450, "manifesto": 18451, "telegraph": 18452}
LIVENESS_ENV = "K6_RECOVERY_LIVENESS"
TLS_MOUNT = {"name": "tls", "mountPath": "/app/certs", "readOnly": True}
CONFIG_MOUNT = {"name": "config", "mountPath": "/app/config", "readOnly": True}
POSTGRES_ENV = [
    {"name": "PGSSLMODE", "value": "verify-full"},
    {"name": "PGSSLROOTCERT", "value": "/app/certs/ca.crt"},
    {"name": "SSL_CERT_FILE", "value": "/app/certs/ca-bundle.crt"},
]
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

    configs, _, _ = inputs.configuration(values)
    config = configs["iam"].replace('tls_enabled = true', 'tls_enabled = false', 1)
    for url in (f"https://iam.{NS}.svc.cluster.local:8443/iam", "https://127.0.0.1:18443/iam"):
        config = config.replace(f'jwks_url = "{url}/.well-known/jwks.json"',
                                f'jwks_url = "{loopback_iam}/.well-known/jwks.json"')
    config = f'internal_service_token = "{campaign.session_replica.liveness_token}"\n' + config
    config += authority_config(loopback_iam)
    pki = campaign.root / "private/pki"
    secret.update({"k6.toml": config, "platform-signing.pem": (pki / "platform-signing.pem").read_bytes(),
                   "platform-signing.pub": (pki / "platform-signing.pub").read_bytes(),
                   LIVENESS_ENV: campaign.session_replica.liveness_token})
    image = campaign.state["images"]["roles"][RECOVERY_ROLE]["tag"]
    campaign.r.cli("kind", ["load", "docker-image", "--name", campaign.state["cluster"], image], timeout=240)

    init = []
    for service, executable in MIGRATIONS.items():
        service_db = database + "_" + service
        campaign.sql(f'CREATE DATABASE "{service_db}";')
        secret[service + "-database-url"] = (
            f"postgresql://postgres:{values['database_password']}@{postgres}:5432/{service_db}"
            "?sslmode=verify-full&sslrootcert=/app/certs/ca.crt"
        )
        init.append(container(service, image, ["/app/bin/" + executable, "up"],
                              [*POSTGRES_ENV, secret_env(name, "DATABASE_URL", service + "-database-url")],
                              [TLS_MOUNT]))
    main = container("monolith", image, ["/app/bin/oodhive-monolith"],
                     service_environment(database, name, set(MIGRATIONS) | {"iam"}),
                     [TLS_MOUNT, CONFIG_MOUNT])
    main["readinessProbe"] = readiness("iam")
    main["ports"] = [{"name": "http", "containerPort": 8080}]
    main["resources"]["limits"]["memory"] = "1536Mi"
    campaign.secret(name, secret)
    deploy(campaign, "monolith", PORT, main, init, [
        tls_volume(campaign),
        config_volume(name, ["k6.toml", "platform-signing.pem", "platform-signing.pub"]),
    ])
    return secret["snapshot-key"]


def service_environment(database, name, services):
    env = [*POSTGRES_ENV, {"name": "RUN_ENV", "value": "k6"}, secret_env(name, LIVENESS_ENV, LIVENESS_ENV)]
    for service in sorted(services):
        prefix = service.upper()
        if service in MIGRATIONS:
            env += [{"name": prefix + "_DATABASE__DB", "value": database + "_" + service},
                    {"name": prefix + "_QUEUE__ENABLED", "value": "false"}]
        if service != "lazaret":
            for field, key in (("STORE_ID", "store-id"), ("AUTHORIZATION_MODEL_ID", "model-id")):
                env.append({"name": f"{prefix}_OPENFGA__{field}", "valueFrom": {
                    "configMapKeyRef": {"name": "local-full-fga", "key": key}}})
    if "telegraph" in services:
        env += [
            {"name": "TELEGRAPH_COMMUNICATION__EMAIL__PROVIDER", "value": "dummy"},
            {"name": "TELEGRAPH_COMMUNICATION__TEMPLATE__TEMPLATE_DIR", "value": "/tmp/k6-telegraph-templates"},
        ]
    if "manifesto" in services:
        env += [
            secret_env(name, "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__HS256_SECRET", "snapshot-key"),
            {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__ISSUER", "value": "aiforall-platform"},
            {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__AUDIENCE", "value": "manifesto-bindings"},
            {"name": "MANIFESTO_APPARATUS__GRANT_SNAPSHOT_AUTH__SERVICE_PRINCIPAL_ISSUER", "value": "aiforall-platform"},
        ]
    return env


def readiness(service):
    return {"httpGet": {"path": f"/{service}/health", "port": 8080},
            "periodSeconds": 2, "timeoutSeconds": 2}


def authority_config(iam):
    return f"""
[auth.session_liveness]
endpoint = "{iam}/internal/auth/session-liveness"
token_env = "{LIVENESS_ENV}"
timeout_ms = 1000
[openfga]
scheme = "https"
host = "openfga.{NS}.svc.cluster.local"
port = 8080
[iam_service]
base_url = "{iam.removesuffix('/iam')}"
api_key = ""
timeout_seconds = 10
[external_provider_service]
base_url = "http://127.0.0.1:8080"
timeout_seconds = 10
max_retries = 0
"""


def create_standalone(campaign, snapshot_key):
    # Each standalone process gets only its verifier material. The platform
    # signing key and the IAM config never enter a companion pod.
    values = campaign.state["values"]
    image = campaign.state["images"]["roles"][RECOVERY_ROLE]["tag"]
    _, _, target = inputs.configuration(values)
    iam = IAM_URL.removesuffix("/internal/auth/session-liveness")
    config = f"""[server]
host = "0.0.0.0"
port = 8080
tls_enabled = false
[auth.jwt]
platform_issuer = "{target['issuer']}"
allow_historical_platform_issuer = false
audience = "{target['audience']}"
jwks_url = "{iam}/.well-known/jwks.json"
allowed_algorithms = ["RS256"]
[database]
host = "postgres.{NS}.svc.cluster.local"
port = 5432
db = "{values['database']}"
[database.creds]
username = "postgres"
password = "{values['database_password']}"
[logging]
level = "error"
filter = "error"
[queue]
enabled = false
""" + authority_config(iam)
    name = campaign.state["name"] + "-companions"
    campaign.secret(name, {"k6.toml": config, "snapshot-key": snapshot_key,
                           LIVENESS_ENV: campaign.session_replica.liveness_token})
    for service, port in STANDALONE_PORTS.items():
        role = "standalone-" + service
        available = service_environment(values["database"], name, {service})
        if service == "hive":
            available.append(secret_env(name, "HIVE_IAM_SERVICE__API_KEY", LIVENESS_ENV))
        main = container(service, image, [f"/app/bin/{service}-service"], available, [TLS_MOUNT, CONFIG_MOUNT])
        main["readinessProbe"] = readiness(service)
        main["ports"] = [{"name": "http", "containerPort": 8080}]
        deploy(campaign, role, port, main, [], [tls_volume(campaign), config_volume(name, ["k6.toml"])])


def create_mesh_companions(campaign):
    image = campaign.state["images"]["roles"][RECOVERY_ROLE]["tag"]
    name = campaign.state["name"] + "-companions"
    database = campaign.state["values"]["database"]
    root = campaign.root / "private/pki"
    for service in COMPANIONS:
        role = "mesh-" + service
        tls_name = campaign.state["name"] + "-" + role + "-tls"
        files = ("ca.crt", "ca-bundle.crt", role + ".crt", role + ".key")
        campaign.secret(tls_name, {file: (root / file).read_bytes() for file in files})
        env = service_environment(database, name, {service})
        prefix = service.upper()
        for option, value in (
            ("TLS_ENABLED", "true"), ("TLS_PORT", "8443"), ("TLS_REQUIRE_CLIENT_CERT", "true"),
            ("TLS_CERT_PATH", f"/app/certs/{role}.crt"), ("TLS_KEY_PATH", f"/app/certs/{role}.key"),
            ("TLS_CLIENT_CA_PATH", "/app/certs/ca.crt"),
        ):
            env.append({"name": f"{prefix}_SERVER__{option}", "value": value})
        env.append({"name": f"{prefix}_AUTH__MESH__TRUSTED_GATEWAY_SAN", "value": "envoy-mesh"})
        if service == "hive":
            env.append(secret_env(name, "HIVE_IAM_SERVICE__API_KEY", LIVENESS_ENV))
        main = container(service, image, [f"/app/bin/{service}-service"], env, [TLS_MOUNT, CONFIG_MOUNT])
        main["readinessProbe"] = readiness(service)
        main["ports"] = [{"name": "https", "containerPort": 8443}]
        tls = config_volume(tls_name, files)
        tls["name"] = "tls"
        deploy(campaign, role, None, main, [], [tls, config_volume(name, ["k6.toml"])])


def secret_env(name, variable, key):
    return {"name": variable, "valueFrom": {"secretKeyRef": {"name": name, "key": key}}}


def config_volume(name, files):
    return {"name": "config", "secret": {"secretName": name, "defaultMode": 0o440,
            "items": [{"key": key, "path": key} for key in files]}}


def tls_volume(campaign):
    volume = config_volume(campaign.state["name"] + "-iam-tls", ["ca.crt", "ca-bundle.crt"])
    volume["name"] = "tls"
    return volume


def deploy(campaign, role, port, main, init, volumes):
    name = campaign.state["name"] + "-" + role
    labels = {"aiforall.dev/k6-run": campaign.state["run_id"], "aiforall.dev/k6-role": role}
    campaign.create({
        "apiVersion": "apps/v1", "kind": "Deployment", "metadata": {"name": name},
        "spec": {"replicas": 1, "selector": {"matchLabels": labels}, "template": {
            "metadata": {"labels": labels | {"aiforall.dev/kind-minimal": "true"}},
            "spec": {"automountServiceAccountToken": False, "securityContext": {
                "runAsNonRoot": True, "runAsUser": 1000, "runAsGroup": 1000, "fsGroup": 1000,
                "seccompProfile": {"type": "RuntimeDefault"}},
                "initContainers": init, "containers": [main], "volumes": volumes},
        }},
    })
    try:
        campaign.rollout(name)
    except Refused as error:
        raise Refused(f"{error}:{role}:{','.join(startup_categories(campaign, role, main['name']))}") from None
    if port is not None:
        campaign.forward(role, port, 8080)


def startup_categories(campaign, role, container_name):
    try:
        pod = campaign.pod(role)
        raw = campaign.r.k(["logs", pod["metadata"]["name"], "--container=" + container_name,
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
