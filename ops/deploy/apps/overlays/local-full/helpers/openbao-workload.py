#!/usr/bin/env python3
"""Local deployment boundary: HTTPS workload login, CAS seed, private snapshots.

No secrets printed, no redirects, no inherited proxy, no raw exception diagnostics.
Not a refresh daemon: only GitHub's OpenBao Agent maintains a token-file.
"""
import json
import os
from pathlib import Path
import re
import ssl
import sys
import time
import urllib.error
import urllib.request

ENDPOINT = "https://openbao.aiforall-local-full.svc.cluster.local:8443"
LIMIT = 65536


class Refused(Exception):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        raise Refused("redirect refused")


class Bao:
    def __init__(self, role):
        context = ssl.create_default_context(cafile="/run/openbao-ca/ca.crt")
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}), NoRedirect(), urllib.request.HTTPSHandler(context=context))
        self.token = None
        jwt = Path("/run/identity/jwt").read_text().strip()
        self.token = self.request("POST", "auth/kubernetes/login", {"role": role, "jwt": jwt})["auth"]["client_token"]
        data = self.request("GET", "auth/token/lookup-self")["data"]
        maximum = 28800 if role == "lazaret" else 300
        if not 30 <= data["ttl"] <= maximum or "root" in data.get("policies", []) + data.get("identity_policies", []):
            raise Refused("workload credential refused")

    def request(self, method, path, body=None, missing=False):
        headers = {"Content-Type": "application/json"}
        if self.token:
            headers["X-Vault-Token"] = self.token
        request = urllib.request.Request(ENDPOINT + "/v1/" + path, method=method, headers=headers,
                                         data=json.dumps(body).encode() if body is not None else None)
        try:
            with self.opener.open(request, timeout=5) as response:
                raw = response.read(LIMIT + 1)
                if len(raw) > LIMIT:
                    raise Refused("response too large")
                return json.loads(raw) if raw else {}
        except urllib.error.HTTPError as error:
            if missing and error.code == 404:
                return None
            raise Refused("OpenBao request refused") from None


def seed(bao, inputs):
    expected = {"version", "client_id", "client_secret", "hmac_secret"}
    if set(inputs) != expected or type(inputs["version"]) is not int or inputs["version"] != 1:
        raise Refused("invalid private seed schema")
    if (not isinstance(inputs["client_secret"], str) or not 1 <= len(inputs["client_secret"]) <= 4096
            or not inputs["client_secret"].strip() or any(ord(c) < 32 for c in inputs["client_secret"])):
        raise Refused("invalid OAuth secret")
    if not isinstance(inputs["hmac_secret"], str) or not re.fullmatch(r"[a-f0-9]{64}", inputs["hmac_secret"]):
        raise Refused("invalid HMAC")
    documents = [("github-connect/oauth", {"client_secret": inputs["client_secret"]}),
                 ("github-connect/iam", {"hmac_secret": inputs["hmac_secret"]})]
    pending = []
    # Validate both before writing either. A 404 data read is NOT proof of absence.
    for path, values in documents:
        metadata = bao.request("GET", "secret/metadata/" + path, missing=True)
        if metadata is None:
            pending.append((path, values))
            continue
        version = metadata["data"]["current_version"]
        latest = metadata["data"]["versions"].get(str(version), {})
        if not version or latest.get("destroyed") or latest.get("deletion_time"):
            raise Refused("deleted/destroyed KV entry; explicit rotation required")
        current = bao.request("GET", "secret/data/" + path)["data"]["data"]
        if current != values:
            raise Refused("KV conflict; explicit rotation required")
    for path, values in pending:
        bao.request("POST", "secret/data/" + path, {"options": {"cas": 0}, "data": values})
    # A partial/CAS-losing write is never rolled back or repaired destructively.


def snapshot(bao, role):
    template = Path("/template/development.toml").read_text()
    if role == "lazaret":
        marker, value = '"__LAZARET_OPENBAO_TOKEN__"', bao.token
    elif role == "iam-hmac":
        marker = '"__GITHUB_OPENBAO_HMAC__"'
        value = bao.request("GET", "secret/data/github-connect/iam")["data"]["data"]["hmac_secret"]
        if not re.fullmatch(r"[a-f0-9]{64}", value):
            raise Refused("invalid HMAC snapshot")
    else:
        raise Refused("unknown snapshot role")
    if template.count(marker) != 1:
        raise Refused("invalid private template")
    rendered = template.replace(marker, json.dumps(value))
    # Parse before publishing; Python 3.12 in the pinned init image has tomllib.
    import tomllib
    tomllib.loads(rendered)
    output = Path("/runtime/development.toml")
    temporary = output.with_suffix(".tmp")
    with temporary.open("w", encoding="utf-8") as target:
        target.write(rendered)
    temporary.chmod(0o600)
    temporary.replace(output)


def main():
    os.umask(0o077)
    role = sys.argv[1]
    deadline = time.monotonic() + 180
    # Startup waits belong here, not in the SDK's 20-second budget. Bounded, silent.
    while True:
        try:
            bao = Bao(role)
            break
        except Exception:
            if time.monotonic() >= deadline:
                raise Refused("workload login deadline exceeded") from None
            time.sleep(2)
    if role == "github-seed":
        seed(bao, json.loads(Path("/inputs/inputs.json").read_text()))
    else:
        snapshot(bao, role)
    print("OpenBao workload bootstrap complete")


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print("OpenBao workload bootstrap refused (details redacted)", file=sys.stderr)
        sys.exit(1)
