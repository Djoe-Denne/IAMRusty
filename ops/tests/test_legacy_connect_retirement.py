"""Offline retirement regressions; no Cargo, daemon, credentials or live inputs."""
import configparser
import json
from pathlib import Path
import tomllib
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ("github-connect", "gitlab-connect")


class LegacyConnectRetirementTests(unittest.TestCase):
    def test_workspace_and_lock_keep_generic_only(self):
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        members = workspace["workspace"]["members"]
        self.assertNotIn("oauth2", workspace["workspace"]["dependencies"])
        self.assertIn("services/OAuthConnect", members)
        for name in ("GitHubConnect", "GitLabConnect"):
            self.assertNotIn("services/" + name, members)
            tree = ROOT / "services" / name
            self.assertFalse(tree.exists() and any(p.is_file() for p in tree.rglob("*")))
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
        names = {p["name"] for p in lock["package"]}
        self.assertIn("oauth-connect-service", names)
        self.assertFalse(any(name.startswith(tuple(p + "-" for p in LEGACY)) for name in names))

    def test_image_inventory_and_chart_keep_generic_only(self):
        inventory = json.loads((ROOT / "ops/deploy/build/targets.json").read_text())["variable"]["inventory"]["default"]
        self.assertIn("oauth", inventory["groups"]["workspace"])
        self.assertIn("oauth", inventory["groups"]["openbao"])
        for name in ("github", "gitlab"):
            self.assertNotIn(name, inventory["targets"])
            self.assertFalse(any(name in group for group in inventory["groups"].values()))
        for target in inventory["targets"].values():
            self.assertTrue((ROOT / target["dockerfile"]).is_file())
        chart = yaml.safe_load((ROOT / "ops/deploy/charts/kind-minimal/values.yaml").read_text())
        # Search the parsed keys, including disabled roles.
        def keys(value):
            if isinstance(value, dict):
                for key, child in value.items():
                    yield key
                    yield from keys(child)
            elif isinstance(value, list):
                for child in value:
                    yield from keys(child)
        all_keys = set(keys(chart))
        self.assertIn("oauth-connect", all_keys)
        self.assertTrue(all(name not in all_keys for name in LEGACY))

    def test_compose_binds_one_generic_service_with_existing_operator_inputs(self):
        services = yaml.safe_load((ROOT / "docker-compose.yml").read_text())["services"]
        self.assertTrue(all(name + "-service" not in services for name in LEGACY))
        oauth = services["oauth-connect-service"]
        self.assertEqual(oauth["build"]["dockerfile"], "services/OAuthConnect/Dockerfile")
        self.assertIn("8085:8080", oauth["ports"])
        self.assertIn("8446:8443", oauth["ports"])
        self.assertNotIn("8086:8080", oauth["ports"])
        depends = services["iam-service"]["depends_on"]
        self.assertIn("oauth-connect-service", depends)
        self.assertTrue(all(name + "-service" not in depends for name in LEGACY))
        mounts = {v["target"]: v for v in oauth["volumes"] if isinstance(v, dict)}
        for target in ("/app/config", "/run/oauth-connect"):
            self.assertTrue(mounts[target]["read_only"])
            self.assertFalse(mounts[target]["bind"]["create_host_path"])
        self.assertIn("../aiforall-oauth-connect/config", mounts["/app/config"]["source"])
        self.assertIn("../aiforall-oauth-connect/runtime", mounts["/run/oauth-connect"]["source"])
        runtime_ignore = (ROOT / "services/OAuthConnect/Dockerfile.dockerignore").read_text().splitlines()
        self.assertIn("**", runtime_ignore)
        self.assertFalse(any(line.startswith("!") for line in runtime_ignore))
        self.assertIn(".local/oauth-connect", (ROOT / "Dockerfile.build.dockerignore").read_text().splitlines())
        self.assertIn("/.local/oauth-connect/", (ROOT / ".gitignore").read_text().splitlines())

    def test_iam_configuration_uses_one_listener_and_instance_paths(self):
        files = [ROOT / "config/development.toml", *sorted((ROOT / "services/IAMRusty/config").glob("*.toml"))]
        checked = 0
        for path in files:
            config = tomllib.loads(path.read_text(encoding="utf-8"))
            connectors = {v["id"]: v for v in config.get("idp", {}).get("connectors", [])}
            if not {"github", "gitlab"} <= connectors.keys():
                continue
            checked += 1
            origins = set()
            for provider in ("github", "gitlab"):
                base = connectors[provider]["base_url"]
                suffix = "/oauth-connect/" + provider
                self.assertTrue(base.endswith(suffix), str(path))
                origins.add(base.removesuffix(suffix))
                for callback in connectors[provider]["redirect_uris"]:
                    self.assertIn("/auth/" + provider + "/", callback)
            self.assertEqual(len(origins), 1, str(path))
        self.assertEqual(checked, 6)

    def test_certificate_inventory_retires_only_vendor_service_identities(self):
        config = configparser.ConfigParser()
        config.read(ROOT / "ops/scripts/local-full-certificates.cnf")
        self.assertIn("oauth-connect", config)
        for name in LEGACY:
            self.assertNotIn(name, config)
            self.assertNotIn("mesh/" + name, config)
        self.assertNotIn("openbao-github", config["bundles"])
        self.assertIn("openbao-oauthconnect", config["bundles"])
        for file in ("ops/deploy/kind-local-pki.ps1", "ops/scripts/generate-platform-mesh-certs.sh"):
            text = (ROOT / file).read_text()
            self.assertTrue(all(name not in text for name in LEGACY))
            self.assertIn("oauth-connect", text)

    def test_existing_github_credential_paths_are_not_retired(self):
        config = tomllib.loads((ROOT / "services/OAuthConnect/configuration/tests/fixtures/ops-connect.toml").read_text())
        github = next(p for p in config["providers"] if p["id"] == "github")
        self.assertEqual(github["client_secret_ref"]["path"], "github-connect/oauth")
        self.assertEqual(github["hmac_secret_ref"]["path"], "github-connect/iam")
        bootstrap = json.loads((ROOT / "ops/deploy/openbao/bootstrap.json").read_text())
        self.assertNotIn("github-connect", bootstrap["roles"])
        self.assertNotIn("local-full-github-connect", bootstrap["policies"])
        self.assertIn("oauth-connect", bootstrap["roles"])
        self.assertIn("secret/data/github-connect/oauth", bootstrap["policies"]["local-full-oauth-connect"])
        self.assertIn("secret/data/github-connect/iam", bootstrap["policies"]["local-full-oauth-connect"])


if __name__ == "__main__":
    unittest.main()
