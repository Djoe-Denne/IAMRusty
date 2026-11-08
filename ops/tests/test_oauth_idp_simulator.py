"""Provider-fixture units and loopback HTTP tests; no product, browser or cluster."""
import base64
import hashlib
import importlib.util
from email.message import Message
import io
from http.client import HTTPConnection
import json
from pathlib import Path
import threading
import unittest
from unittest.mock import Mock
from urllib.parse import parse_qs, urlencode, urlsplit


def load():
    spec = importlib.util.spec_from_file_location(
        "oauth_simulator", Path(__file__).with_name("support") / "oauth-idp-simulator.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SimulatorTests(unittest.TestCase):
    def setUp(self):
        self.module = load()
        self.now = 100
        self.provider = self.module.Provider("proofclient", "private-client-secret", "proofrun123456",
                                             clock=lambda: self.now)
        self.verifier = "a" * 43
        self.redirect = "http://127.0.0.1:18080/iam/api/auth/gitlab/callback"
        self.authorization = dict(response_type="code", client_id="proofclient",
                                  redirect_uri=self.redirect, state="opaque-state",
                                  code_challenge=base64.urlsafe_b64encode(hashlib.sha256(
                                      self.verifier.encode()).digest()).rstrip(b"=").decode(),
                                  code_challenge_method="S256")

    def exchange(self, code, **changes):
        values = dict(grant_type="authorization_code", code=code, client_id="proofclient",
                      client_secret="private-client-secret", redirect_uri=self.redirect,
                      code_verifier=self.verifier)
        return self.provider.exchange({**values, **changes})

    def test_simulator_consumes_code_once_and_verifies_s256_client_redirect(self):
        for changes in ({"client_id": "foreign"}, {"client_secret": "wrong"},
                        {"redirect_uri": self.redirect + "/"}, {"code_verifier": "b" * 43}):
            with self.subTest(changes=tuple(changes)):
                code = self.provider.authorize(self.authorization)
                with self.assertRaises(self.module.Rejected):
                    self.exchange(code, **changes)
                with self.assertRaises(self.module.Rejected):
                    self.exchange(code)
        code = self.provider.authorize(self.authorization)
        token = self.exchange(code)
        self.assertEqual(token["token_type"], "Bearer")
        with self.assertRaises(self.module.Rejected):
            self.exchange(code)

    def test_simulator_receives_real_authorize_token_profile_without_secret_output(self):
        # Protocol functions used by the real HTTP handler. This is fixture-only,
        # not evidence that GitLabConnect or a browser actually called it.
        code = self.provider.authorize(self.authorization)
        token = self.exchange(code)["access_token"]
        user = self.provider.profile("Bearer " + token)
        self.assertTrue(user["confirmed_at"])
        self.assertEqual(user["email"], "proofrun123456@example.invalid")
        same_run = self.module.Provider("proofclient", "private-client-secret", "proofrun123456")
        other_run = self.module.Provider("proofclient", "private-client-secret", "proofrun654321")
        self.assertEqual(user["id"], same_run.user_id)
        self.assertNotEqual(user["id"], other_run.user_id)
        report = self.provider.observations()
        self.assertEqual(report, dict(run_id="proofrun123456", authorize=1, token=1,
                                      profile=1, pkce_validated=1))
        for sensitive in (code, token, self.verifier, "private-client-secret", "opaque-state"):
            self.assertNotIn(sensitive, str(report))

    def test_expiration_bad_authorization_and_unknown_token_refused(self):
        for change in ({"response_type": "token"}, {"code_challenge_method": "plain"},
                       {"redirect_uri": self.redirect.replace("127.0.0.1", "localhost")},
                       {"code_challenge": "bad"}, {"state": ""}):
            with self.assertRaises(self.module.Rejected):
                self.provider.authorize({**self.authorization, **change})
        code = self.provider.authorize(self.authorization)
        self.now += 121
        with self.assertRaises(self.module.Rejected):
            self.exchange(code)
        with self.assertRaises(self.module.Rejected):
            self.provider.profile("Bearer unknown")

    def test_http_handler_matches_basic_client_token_and_profile_protocol(self):
        handler = self.module.handler(self.provider).__new__(self.module.handler(self.provider))
        handler.reply = Mock()
        handler.path = "/oauth/authorize?" + urlencode(self.authorization)
        handler.do_GET()
        status, _, redirect = handler.reply.call_args.args
        self.assertEqual(status, 303)
        code = parse_qs(urlsplit(redirect).query)["code"][0]
        body = urlencode(dict(grant_type="authorization_code", code=code,
                              redirect_uri=self.redirect, code_verifier=self.verifier)).encode()
        handler.path = "/oauth/token"
        handler.headers = Message()
        handler.headers["Content-Type"] = "application/x-www-form-urlencoded"
        handler.headers["Content-Length"] = str(len(body))
        handler.headers["Authorization"] = "Basic " + base64.b64encode(
            b"proofclient:private-client-secret").decode()
        handler.rfile = io.BytesIO(body)
        handler.do_POST()
        status, payload = handler.reply.call_args.args
        self.assertEqual(status, 200)
        handler.path = "/api/v4/user"
        handler.headers.replace_header("Authorization", "Bearer " + payload["access_token"])
        handler.do_GET()
        self.assertEqual(handler.reply.call_args.args[0], 200)
        self.assertEqual(self.provider.observations()["pkce_validated"], 1)

    def test_http_handler_rejects_oversized_and_duplicate_form_before_exchange(self):
        for body, length in ((b"code=a&code=b", "13"), (b"", "8193")):
            handler = self.module.handler(self.provider).__new__(self.module.handler(self.provider))
            handler.reply = Mock()
            handler.path = "/oauth/token"
            handler.headers = Message()
            handler.headers["Content-Type"] = "application/x-www-form-urlencoded"
            handler.headers["Content-Length"] = length
            handler.rfile = io.BytesIO(body)
            handler.do_POST()
            self.assertEqual(handler.reply.call_args.args, (400, {"error": "invalid_grant"}))
            self.assertEqual(self.provider.observations()["token"], 0)


class SimulatedProviderHttpTests(unittest.TestCase):
    """Exercise the deployed handler protocol, not a mocked reply method."""

    def setUp(self):
        self.module = load()
        self.client_id = "simulatedclient"
        self.client_secret = "expected-simulated-client-secret"
        self.run_id = "simulatedrun123456"
        self.redirect = "https://127.0.0.1:18080/iam/api/auth/simulated/callback"
        self.verifier = "a" * 43
        self.provider = self.module.Provider(self.client_id, self.client_secret, self.run_id,
                                             instance="simulated")
        self.server = self.module.ThreadingHTTPServer(
            ("127.0.0.1", 0), self.module.handler(self.provider))
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever,
                                       kwargs={"poll_interval": 0.01}, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)
        self.assertFalse(self.thread.is_alive(), "simulated HTTP fixture must stop")

    def request(self, method, path, values=None, authorization=None):
        connection = HTTPConnection("127.0.0.1", self.server.server_port, timeout=2)
        headers = {"Connection": "close"}
        body = None
        if values is not None:
            body = urlencode(values).encode()
            headers["Content-Type"] = "application/x-www-form-urlencoded"
        if authorization is not None:
            headers["Authorization"] = authorization
        try:
            connection.request(method, path, body=body, headers=headers)
            response = connection.getresponse()
            raw = response.read(4097)
            self.assertLessEqual(len(raw), 4096, "fixture response must remain bounded")
            return response.status, dict(response.getheaders()), json.loads(raw)
        finally:
            connection.close()

    def authorize(self):
        values = dict(response_type="code", client_id=self.client_id,
                      redirect_uri=self.redirect, state="simulated-http-state",
                      code_challenge=base64.urlsafe_b64encode(hashlib.sha256(
                          self.verifier.encode()).digest()).rstrip(b"=").decode(),
                      code_challenge_method="S256")
        status, headers, _ = self.request("GET", "/oauth/authorize?" + urlencode(values))
        self.assertEqual(status, 303)
        location = urlsplit(headers["Location"])
        self.assertEqual(location.scheme + "://" + location.netloc + location.path, self.redirect)
        query = parse_qs(location.query)
        self.assertEqual(query["state"], [values["state"]])
        self.assertEqual(len(query["code"]), 1)
        return query["code"][0]

    def token_values(self, code):
        return dict(grant_type="authorization_code", code=code, client_id=self.client_id,
                    client_secret=self.client_secret, redirect_uri=self.redirect,
                    code_verifier=self.verifier)

    def test_simulated_accepts_configured_client_secret_post_over_http(self):
        status, _, payload = self.request("POST", "/oauth/token", self.token_values(self.authorize()))
        self.assertEqual(status, 200)
        self.assertEqual(payload["token_type"], "Bearer")
        self.assertTrue(payload["access_token"])
        status, _, profile = self.request("GET", "/user", authorization="Bearer " + payload["access_token"])
        self.assertEqual(status, 200)
        self.assertEqual(profile["email"], self.run_id + "@example.invalid")
        status, _, counts = self.request("GET", "/observations")
        self.assertEqual(status, 200)
        self.assertEqual(counts, dict(run_id=self.run_id, authorize=1, token=1, profile=1, pkce_validated=1))

    def test_wrong_or_missing_client_secret_returns_400_without_token_and_burns_code(self):
        for supplied in ("wrong-simulated-client-secret", None):
            with self.subTest(missing=supplied is None):
                code = self.authorize()
                values = self.token_values(code)
                if supplied is None:
                    del values["client_secret"]
                else:
                    values["client_secret"] = supplied
                status, _, payload = self.request("POST", "/oauth/token", values)
                self.assertEqual((status, payload), (400, {"error": "invalid_grant"}))
                # Even the correct secret cannot replay a code consumed by refusal.
                status, _, payload = self.request("POST", "/oauth/token", self.token_values(code))
                self.assertEqual((status, payload), (400, {"error": "invalid_grant"}))
                _, _, counts = self.request("GET", "/observations")
                self.assertEqual((counts["token"], counts["profile"], counts["pkce_validated"]), (0, 0, 0))
        # The same valid request with a fresh code succeeds: the negative oracle
        # above is not merely an unreachable service or an invalid PKCE fixture.
        status, _, payload = self.request("POST", "/oauth/token", self.token_values(self.authorize()))
        self.assertEqual(status, 200)
        self.assertTrue(payload["access_token"])

    def test_wrong_access_token_returns_400_without_profile(self):
        status, _, payload = self.request("GET", "/user", authorization="Bearer unknown-test-token")
        self.assertEqual((status, payload), (400, {"error": "invalid_request"}))
        _, _, counts = self.request("GET", "/observations")
        self.assertEqual((counts["token"], counts["profile"]), (0, 0))


if __name__ == "__main__":
    unittest.main()
