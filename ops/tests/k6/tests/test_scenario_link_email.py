"""Offline contracts for the hosted explicit LINK fixture; no deployed verdict."""
import base64
import hashlib
import importlib.util
from pathlib import Path
import tomllib
import unittest

import yaml

K6 = Path(__file__).resolve().parents[1]
RUN = "a" * 32
ORIGIN = "https://127.0.0.1:18443"
GITLAB = ORIGIN + "/iam/api/auth/gitlab/callback"
GITHUB = ORIGIN + "/iam/api/auth/github/callback"
GITHUB_RELINK = ORIGIN + "/iam/api/auth/github/relink-callback"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class LinkEmailFixtureTests(unittest.TestCase):
    def setUp(self):
        self.simulator = load("link_email_simulator", K6.parent / "support/oauth-idp-simulator.py")
        self.inputs = load("link_email_inputs", K6 / "ci/inputs.py")
        self.values = self.inputs.fresh(RUN)
        self.verifier = "a" * 43
        self.authorization = dict(
            response_type="code", client_id=self.values["client_id"], state="opaque-state",
            code_challenge_method="S256", code_challenge=base64.urlsafe_b64encode(
                hashlib.sha256(self.verifier.encode()).digest()).rstrip(b"=").decode(),
        )

    def provider(self):
        _, config, _ = self.inputs.configuration(self.values)
        return self.simulator.Provider(config["client_id"], config["client_secret"], RUN,
            edge_origin=config["edge_origin"], fixture_profile=config.get("fixture_profile"))

    def authorize(self, provider, redirect, **selector):
        return provider.authorize({**self.authorization, "redirect_uri": redirect, **selector})

    def exchange(self, provider, redirect, code, **changes):
        values = dict(grant_type="authorization_code", code=code, redirect_uri=redirect,
                      code_verifier=self.verifier, client_id=self.values["client_id"],
                      client_secret=self.values["client_secret"])
        return provider.exchange({**values, **changes})["access_token"]

    def test_mounted_profile_preserves_browser_and_binds_each_exact_email_and_proof(self):
        provider = self.provider()
        fixtures = [(GITLAB, {}, f"{RUN}@example.invalid", True)]
        for kind, verification in (("attested", True), ("false", False), ("omitted", None)):
            for redirect, selector, verified in (
                (GITLAB, "owner-" + kind, True), (GITHUB, "link-" + kind, verification),
            ):
                fixtures.append((redirect, {"fixture_account": selector}, f"{RUN}-{selector}@example.invalid", verified))

        # Authorize all of them first, then complete in reverse order. A global
        # 'last selected identity' would mix the addresses or attestations.
        codes = [(self.authorize(provider, redirect, **selector), redirect, email, verified)
                 for redirect, selector, email, verified in fixtures]
        subjects = set()
        for code, redirect, email, verified in reversed(codes):
            token = self.exchange(provider, redirect, code)
            profile = provider.profile("Bearer " + token)
            with self.subTest(email=email):
                self.assertEqual(profile["email"], email)
                if verified is None:
                    self.assertNotIn("email_verified", profile)
                else:
                    self.assertIs(profile["email_verified"], verified)
                subjects.add(profile["id"])
        self.assertEqual(len(subjects), len(fixtures))
        self.assertEqual(provider.observations(), {
            "run_id": RUN, "fixture_profile": "hosted-link-email",
            **dict.fromkeys(("authorize", "token", "profile", "pkce_validated"), len(fixtures)),
        })

    def test_hosted_profile_refuses_non_hosted_origin_instance_or_run_id(self):
        provider = self.provider()
        for changed in ({}, {"edge_origin": "http://127.0.0.1:18080"},
                        {"edge_origin": ORIGIN, "instance": "simulated"}):
            with self.subTest(changed=changed), self.assertRaises(self.simulator.Rejected):
                self.simulator.Provider(provider.client_id, provider.secret, RUN,
                                        fixture_profile="hosted-link-email", **changed)
        with self.assertRaises(self.simulator.Rejected):
            self.simulator.Provider(provider.client_id, provider.secret, "nonhexhosted123456",
                                    edge_origin=ORIGIN, fixture_profile="hosted-link-email")

    def test_only_closed_authorize_choices_and_exact_redirects_are_admitted(self):
        provider = self.provider()
        for redirect, selector in ((GITHUB, {}), (GITLAB, {"fixture_account": "link-attested"}),
                                   (GITHUB, {"fixture_account": "owner-attested"}),
                                   (GITHUB, {"fixture_account": "external@example.com"}),
                                   (GITHUB_RELINK, {"fixture_account": "link-attested"}),
                                   (GITHUB + "/", {"fixture_account": "link-attested"})):
            with self.subTest(redirect=redirect, selector=selector), self.assertRaises(self.simulator.Rejected):
                self.authorize(provider, redirect, **selector)
        for redirect, changes in ((GITLAB, {}), (GITHUB, {"fixture_account": "link-attested"})):
            code = self.authorize(provider, GITHUB, fixture_account="link-attested")
            with self.assertRaises(self.simulator.Rejected):
                self.exchange(provider, redirect, code, **changes)
            with self.assertRaises(self.simulator.Rejected):
                self.exchange(provider, GITHUB, code)

    def test_hosted_configuration_uses_real_second_connector_without_exempting_link_start(self):
        config, provider, target = self.inputs.configuration(self.values)
        iam = tomllib.loads(config["iam"])["idp"]["connectors"]
        oauth = tomllib.loads(config["oauth"])["providers"]
        self.assertEqual([row["id"] for row in iam], ["gitlab", "github"])
        self.assertEqual([row["id"] for row in oauth], ["gitlab", "github"])
        self.assertEqual(iam[1]["redirect_uris"], [GITHUB, GITHUB_RELINK])
        self.assertTrue(iam[1]["base_url"].endswith("/oauth-connect/github"))
        self.assertEqual(oauth[1]["redirect_uris"], iam[1]["redirect_uris"])
        self.assertEqual(oauth[1]["transport_security"], "verified")
        self.assertEqual(oauth[1]["hmac_secret"], iam[1]["hmac_secret"])
        self.assertEqual(oauth[1]["client_id"], provider["client_id"])
        self.assertEqual(oauth[1]["profile"]["authorization_url"], target["provider"])
        self.assertEqual(oauth[1]["profile"]["claims"]["email_verified"],
                         {"api": "user", "json_path": "$.email_verified"})
        self.assertTrue(oauth[1]["pkce_supported"] and iam[1]["pkce_supported"])

        envoy = yaml.safe_load((K6 / "ci/config/envoy.yaml").read_text(encoding="utf-8"))
        manager = envoy["static_resources"]["listeners"][0]["filter_chains"][0]["filters"][0]["typed_config"]
        routes = manager["route_config"]["virtual_hosts"][0]["routes"]
        callback = [row for row in routes if row["match"].get("path") == "/iam/api/auth/github/callback"]
        self.assertEqual(len(callback), 1)
        self.assertEqual(callback[0]["match"]["headers"], [{"name": ":method", "string_match": {"exact": "GET"}}])
        self.assertTrue(callback[0]["typed_per_filter_config"]["envoy.filters.http.ext_authz"]["disabled"])
        self.assertFalse(any(row["match"].get("path") in {
            "/iam/api/auth/github/link", "/iam/api/auth/github/relink-callback",
        } for row in routes))
        self.assertNotIn("typed_per_filter_config", routes[-1])
