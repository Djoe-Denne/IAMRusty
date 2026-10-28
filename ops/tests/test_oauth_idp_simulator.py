"""Provider-fixture units only; no product, browser, cluster or socket."""
import base64
import hashlib
import importlib.util
from email.message import Message
import io
from pathlib import Path
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


if __name__ == "__main__":
    unittest.main()
