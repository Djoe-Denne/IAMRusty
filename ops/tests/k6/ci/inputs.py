"""Public profile plus fresh private values; no operator configuration import."""
from pathlib import Path
import re
import secrets
from string import Template
import tomllib

HERE = Path(__file__).resolve().parent
FIELDS = frozenset(("run_id", "name", "database", "database_password", "hmac_secret",
                    "oauth_state_secret", "client_id", "client_secret", "sqs_access_key", "sqs_secret_key"))


def fresh(run_id):
    if not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id):
        raise ValueError("CI_RUN_ID")
    return dict(run_id=run_id, name="k6-" + run_id, database="k6_" + run_id,
                **{name: secrets.token_hex(32) for name in FIELDS - {"run_id", "name", "database"}})


def configuration(values):
    if not isinstance(values, dict) or values.keys() != FIELDS:
        raise ValueError("CI_INPUT_FIELDS")
    run_id = values["run_id"]
    if (not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id)
            or values["name"] != "k6-" + run_id or values["database"] != "k6_" + run_id):
        raise ValueError("CI_INPUT_IDENTITY")
    for name in FIELDS - {"run_id", "name", "database"}:
        if not isinstance(values[name], str) or not re.fullmatch(r"[a-f0-9]{64}", values[name]):
            raise ValueError("CI_INPUT_SECRET_FORMAT")
    configs = {}
    for role in ("iam", "oauth"):
        template = (HERE / "config" / f"{role}.toml.in").read_text(encoding="utf-8")
        text = Template(template).substitute(values)
        tomllib.loads(text)  # Fail before creating a workload, never salvage TOML.
        configs[role] = text
    provider = dict(run_id=run_id, client_id=values["client_id"],
                    client_secret=values["client_secret"], edge_origin="https://127.0.0.1:18443",
                    tls_cert_file="/app/certs/idp.crt", tls_key_file="/app/certs/idp.key")
    target = dict(version=1, profile="iam-native-parity", run_id=run_id,
                  origin="https://127.0.0.1:18443", provider="https://localhost:18445/oauth/authorize",
                  issuer="https://127.0.0.1:18443/iam", audience="aiforall")
    return configs, provider, target
