"""Synthetic GitLab provider behind the REAL connector; never emits IAM JWTs.

Parent-run only. Credentials come from a private mounted JSON file, never argv.
No request/URL/header logging. Observations contain counters and run ID only.
"""
import base64
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import secrets
import threading
import time
from urllib.parse import parse_qs, unquote_plus, urlencode, urlsplit

REDIRECTS = frozenset("http://127.0.0.1:18080/iam/api/auth/gitlab/" + path
                      for path in ("callback", "relink-callback"))


class Rejected(ValueError):
    def __init__(self):
        super().__init__("synthetic provider request rejected")


class Provider:
    def __init__(self, client_id, client_secret, run_id, clock=time.monotonic):
        if (not re.fullmatch(r"[a-z0-9]{12,32}", run_id)
                or not re.fullmatch(r"[A-Za-z0-9_-]{8,128}", client_id)
                or not isinstance(client_secret, str) or not 16 <= len(client_secret) <= 256):
            raise Rejected()
        self.client_id, self.secret, self.run_id = client_id, client_secret, run_id
        self.user_id = int.from_bytes(hashlib.sha256(run_id.encode()).digest()[:8], "big") & ((1 << 63) - 1)
        self.clock, self.lock = clock, threading.Lock()
        self.codes, self.tokens = {}, {}
        self.counts = dict(authorize=0, token=0, profile=0, pkce_validated=0)

    def _purge(self):
        now = self.clock()
        self.codes = {key: value for key, value in self.codes.items() if value[0] > now}
        self.tokens = {key: expiry for key, expiry in self.tokens.items() if expiry > now}

    def authorize(self, values):
        if (values.get("response_type") != "code" or values.get("client_id") != self.client_id
                or values.get("redirect_uri") not in REDIRECTS
                or values.get("code_challenge_method") != "S256"
                or not re.fullmatch(r"[A-Za-z0-9_-]{43}", values.get("code_challenge", ""))
                or not 1 <= len(values.get("state", "")) <= 2048):
            raise Rejected()
        with self.lock:
            self._purge()
            if len(self.codes) >= 128:
                raise Rejected()
            code = secrets.token_urlsafe(32)
            self.codes[code] = (self.clock() + 120, values["redirect_uri"], values["code_challenge"])
            self.counts["authorize"] += 1
            return code

    def exchange(self, values):
        with self.lock:
            self._purge()
            # A failed binding attempt burns the code too; no retry oracle.
            record = self.codes.pop(values.get("code", ""), None)
            verifier = values.get("code_verifier", "")
            challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
            if (not record or values.get("grant_type") != "authorization_code"
                    or values.get("client_id") != self.client_id
                    or not hmac.compare_digest(values.get("client_secret", "").encode(), self.secret.encode())
                    or values.get("redirect_uri") != record[1]
                    or not re.fullmatch(r"[A-Za-z0-9._~-]{43,128}", verifier)
                    or not hmac.compare_digest(challenge, record[2]) or len(self.tokens) >= 128):
                raise Rejected()
            token = secrets.token_urlsafe(32)
            self.tokens[token] = self.clock() + 300
            self.counts["token"] += 1
            self.counts["pkce_validated"] += 1
            return dict(access_token=token, token_type="Bearer", expires_in=300, scope="read_user")

    def profile(self, authorization):
        with self.lock:
            self._purge()
            if not authorization.startswith("Bearer ") or authorization[7:] not in self.tokens:
                raise Rejected()
            self.counts["profile"] += 1
            return dict(id=self.user_id, username=self.run_id, name="OAuth proof account",
                        email=self.run_id + "@example.invalid", confirmed_at="2026-10-28T00:00:00Z")

    def observations(self):
        with self.lock:
            return dict(run_id=self.run_id, **self.counts)


def fields(raw):
    parsed = parse_qs(raw, keep_blank_values=True, strict_parsing=True, max_num_fields=16)
    if any(len(values) != 1 for values in parsed.values()):
        raise Rejected()
    return {key: values[0] for key, values in parsed.items()}


def handler(provider):
    class Handler(BaseHTTPRequestHandler):
        server_version = "SyntheticOAuth"

        def log_message(self, *_):
            pass

        def setup(self):
            super().setup()
            self.connection.settimeout(5)

        def reply(self, status, value, location=None):
            body = json.dumps(value).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Referrer-Policy", "no-referrer")
            self.send_header("Content-Length", str(len(body)))
            if location:
                self.send_header("Location", location)
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            try:
                if len(self.path) > 8192:
                    raise Rejected()
                url = urlsplit(self.path)
                if url.scheme or url.netloc or url.fragment:
                    raise Rejected()
                if url.path == "/oauth/authorize":
                    values = fields(url.query)
                    code = provider.authorize(values)
                    self.reply(303, {}, values["redirect_uri"] + "?" + urlencode(dict(code=code, state=values["state"])))
                elif url.path == "/api/v4/user" and not url.query:
                    self.reply(200, provider.profile(self.headers.get("Authorization", "")))
                elif url.path == "/observations" and not url.query:
                    self.reply(200, provider.observations())
                else:
                    raise Rejected()
            except (ValueError, UnicodeError):
                self.reply(400, dict(error="invalid_request"))

        def do_POST(self):
            try:
                if (self.path != "/oauth/token" or self.headers.get("Transfer-Encoding")
                        or self.headers.get("Content-Type", "").split(";")[0] != "application/x-www-form-urlencoded"):
                    raise Rejected()
                lengths = self.headers.get_all("Content-Length", [])
                if len(lengths) != 1 or not re.fullmatch(r"[0-9]{1,5}", lengths[0]):
                    raise Rejected()
                size = int(lengths[0])
                if not 1 <= size <= 8192:
                    raise Rejected()
                raw = self.rfile.read(size)
                if len(raw) != size:
                    raise Rejected()
                values = fields(raw.decode("utf-8"))
                authorization = self.headers.get_all("Authorization", [])
                # oauth2::BasicClient defaults to HTTP Basic client auth. Accept
                # that real connector protocol, never ambiguous double auth.
                if authorization:
                    if len(authorization) != 1 or not authorization[0].startswith("Basic ") or "client_secret" in values:
                        raise Rejected()
                    credentials = base64.b64decode(authorization[0][6:], validate=True).decode("utf-8").split(":")
                    if len(credentials) != 2:
                        raise Rejected()
                    client_id, client_secret = map(unquote_plus, credentials)
                    if "client_id" in values and values["client_id"] != client_id:
                        raise Rejected()
                    values.update(client_id=client_id, client_secret=client_secret)
                self.reply(200, provider.exchange(values))
            except (ValueError, UnicodeError):
                self.close_connection = True
                self.reply(400, dict(error="invalid_grant"))

    return Handler


def main():
    try:
        inputs = json.loads(Path(os.environ["OAUTH_PROOF_INPUT_FILE"]).read_text(encoding="utf-8"))
        provider = Provider(inputs["client_id"], inputs["client_secret"], inputs["run_id"])
        server = ThreadingHTTPServer(("0.0.0.0", 8080), handler(provider))
        server.daemon_threads = True
        server.serve_forever()
    except Exception:
        # No traceback may disclose credentials or request bodies.
        raise SystemExit("synthetic provider stopped; details redacted") from None


if __name__ == "__main__":
    main()
