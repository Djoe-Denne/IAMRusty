"""Offline private input generation and configuration contracts."""
import importlib.util
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[4]
SPEC = importlib.util.spec_from_file_location("ci_inputs", ROOT / "ops/tests/k6/ci/inputs.py")
i = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(i)
RUN = "a" * 32


class InputsTests(unittest.TestCase):
    def test_fresh_secrets_even_with_same_run_id(self):
        first, second = i.fresh(RUN), i.fresh(RUN)
        for field in i.FIELDS - {"run_id", "name", "database"}:
            self.assertNotEqual(first[field], second[field], field)
            self.assertEqual(len(first[field]), 64)
        self.assertEqual(first["name"], "k6-" + RUN)
        self.assertEqual(first["database"], "k6_" + RUN)

    def test_invalid_run_ids_cannot_become_sql_or_kubernetes_names(self):
        for value in (None, True, "", "A" * 32, "../escape", "a" * 31, "a" * 33,
                      "a" * 31 + "'", "a" * 32 + "\n"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                i.fresh(value)

    def test_injection_extra_missing_or_cross_run_values_are_rejected(self):
        base = i.fresh(RUN)
        variants = [{**base, "unexpected": "secret"}, {**base, "name": "foreign"},
                    {**base, "database": "shared"}, {**base, "run_id": "b" * 32}]
        for field in i.FIELDS:
            variants.append({k: v for k, v in base.items() if k != field})
        for field in i.FIELDS - {"run_id", "name", "database"}:
            variants.append({**base, field: '"\n[unsafe]\nkey="value'})
        for value in variants:
            with self.assertRaises(ValueError):
                i.configuration(value)

    def test_real_oauth_profile_hmac_pkce_and_fresh_database(self):
        values = i.fresh(RUN)
        configs, provider, target = i.configuration(values)
        iam, oauth = (tomllib.loads(configs[role]) for role in ("iam", "oauth"))
        self.assertTrue(iam["server"]["tls_enabled"])
        self.assertEqual(iam["database"]["db"], "k6_" + RUN)
        self.assertEqual(iam["database"]["creds"]["password"], values["database_password"])
        self.assertEqual(iam["jwt"]["oauth_state_secret"], values["oauth_state_secret"])
        self.assertEqual(iam["jwt"]["secret"]["key_id"], RUN)
        connector = iam["idp"]["connectors"][0]
        self.assertEqual(len(iam["idp"]["connectors"]), 1)
        self.assertTrue(connector["pkce_supported"])
        self.assertEqual(connector["hmac_secret"], values["hmac_secret"])
        profile = oauth["providers"][0]
        self.assertEqual(len(oauth["providers"]), 1)
        self.assertEqual(profile["hmac_secret"], connector["hmac_secret"])
        self.assertEqual(profile["redirect_uris"], connector["redirect_uris"])
        self.assertEqual(profile["client_secret"], provider["client_secret"])
        self.assertEqual(profile["transport_security"], "verified")
        self.assertEqual(profile["secrets_mode"], "direct")
        self.assertTrue(profile["pkce_supported"])
        self.assertEqual(profile["profile"]["claims"]["email_verified"]["json_path"], "$.email_verified")
        self.assertEqual(profile["profile"]["authorization_url"], target["provider"])
        self.assertNotIn("secret", " ".join(target))
        self.assertNotIn("localhost:18445", target["origin"])
        self.assertEqual(target["profile"], "iam-native-parity")
        self.assertNotIn("openbao", oauth)


if __name__ == "__main__":
    unittest.main()
