import base64
import hmac
import json
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, call

import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from binaries import RECOVERY_ROLE, ROOT, inventory
import inputs
from pki import COMPANIONS, identities, mesh_hostname
import recovery_mesh
from recovery_monolith import LIVENESS_ENV, PORT, STANDALONE_PORTS, classify_startup, create_monolith, create_standalone
from recovery_monolith import snapshot_fixture
from session_replica import IAM_URL


class RecoveryMonolithTests(unittest.TestCase):
    def test_monolith_uses_the_same_writer_and_separate_companion_databases(self):
        run_id = "a" * 32
        values = inputs.fresh(run_id)
        image = "local/k6-recovery:k6-123-1"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pki = root / "private/pki"
            pki.mkdir(parents=True)
            for name in ("platform-signing.pem", "platform-signing.pub"):
                (pki / name).write_bytes(b"fixture")
            campaign = Mock(root=root, state={"values": values, "name": values["name"], "run_id": run_id,
                                             "cluster": "k6-e2e-123-1",
                                             "images": {"roles": {RECOVERY_ROLE: {"tag": image}}}},
                            session_replica=SimpleNamespace(liveness_token="run-owned-capability"))
            snapshot_key = create_monolith(campaign)

        name, secret = campaign.secret.call_args.args
        self.assertEqual(snapshot_key, secret["snapshot-key"])
        deployment, = campaign.create.call_args.args
        self.assertEqual(deployment["metadata"]["name"], name)
        pod = deployment["spec"]["template"]["spec"]
        self.assertFalse(pod["automountServiceAccountToken"])
        config = tomllib.loads(secret["k6.toml"])
        self.assertEqual(config["database"]["db"], values["database"])
        self.assertEqual(config["auth"]["jwt"]["jwks_url"], "http://127.0.0.1:8080/iam/.well-known/jwks.json")
        self.assertEqual(config["auth"]["session_liveness"]["endpoint"],
                         "http://127.0.0.1:8080/iam/internal/auth/session-liveness")
        self.assertEqual(config["iam_service"]["base_url"], "http://127.0.0.1:8080")
        self.assertEqual(config["external_provider_service"]["max_retries"], 0)
        main, = pod["containers"]
        self.assertEqual(main["command"], ["/app/bin/oodhive-monolith"])
        self.assertFalse(any(entry.get("value") == secret[LIVENESS_ENV] for entry in main["env"]))
        migration_names = {name for package, name in inventory(ROOT, RECOVERY_ROLE).items() if package.endswith("-migration")}
        self.assertEqual({entry["command"][0].split("/")[-1] for entry in pod["initContainers"]}, migration_names)
        for migration in pod["initContainers"]:
            service = migration["name"]
            database = values["database"] + "_" + service
            self.assertIn(f"/{database}?sslmode=verify-full", secret[service + "-database-url"])
            self.assertIn({"name": service.upper() + "_DATABASE__DB", "value": database}, main["env"])
        campaign.forward.assert_called_once_with("monolith", PORT, 8080)

    def test_standalone_services_get_authoritative_tls_verifiers_without_the_signing_key(self):
        values = inputs.fresh("a" * 32)
        image = "local/k6-recovery:k6-123-1"
        campaign = Mock(state={"values": values, "name": values["name"], "run_id": values["run_id"],
                               "images": {"roles": {RECOVERY_ROLE: {"tag": image}}}},
                        session_replica=SimpleNamespace(liveness_token="run-owned-capability"))
        create_standalone(campaign, "snapshot-test-key")

        name, secret = campaign.secret.call_args.args
        self.assertEqual(set(secret), {"k6.toml", "snapshot-key", LIVENESS_ENV})
        config = tomllib.loads(secret["k6.toml"])
        self.assertEqual(config["auth"]["session_liveness"]["endpoint"], IAM_URL)
        self.assertNotIn("jwt", config)
        self.assertEqual(len(campaign.create.call_args_list), len(STANDALONE_PORTS))
        for invocation, (service, port) in zip(campaign.create.call_args_list, STANDALONE_PORTS.items(), strict=True):
            deployment, = invocation.args
            pod = deployment["spec"]["template"]["spec"]
            main, = pod["containers"]
            self.assertEqual(main["command"], [f"/app/bin/{service}-service"])
            self.assertEqual(pod["initContainers"], [])
            self.assertFalse(pod["automountServiceAccountToken"])
            self.assertIn({"name": LIVENESS_ENV, "valueFrom": {"secretKeyRef": {"name": name, "key": LIVENESS_ENV}}},
                          main["env"])
            self.assertIn(call(f"standalone-{service}", port, 8080), campaign.forward.call_args_list)

    def test_snapshot_uses_the_run_owned_database_and_workload_audience(self):
        sql = Mock()
        owner = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
        database = "k6_" + "a" * 32 + "_manifesto"
        key = "fixture-key"
        path, project, component, token = snapshot_fixture(sql, database, owner, key)
        self.assertEqual(path, f"/api/projects/{project}/bindings/{component}")
        statement, target = sql.call_args.args
        self.assertEqual(target, database)
        for relation in ("projects", "project_components", "apparatus_bindings"):
            self.assertIn("INSERT INTO public." + relation, statement)
        message, signature = token.rsplit(".", 1)
        expected = base64.urlsafe_b64encode(hmac.digest(key.encode(), message.encode(), "sha256")).rstrip(b"=")
        self.assertEqual(signature.encode(), expected)
        claims = json.loads(base64.urlsafe_b64decode(message.split(".")[1] + "=="))
        self.assertEqual(claims["aud"], "manifesto-bindings")
        self.assertNotEqual(claims["sub"], owner)

    def test_startup_receipt_exports_only_closed_categories(self):
        output = b"Failed to deserialize configuration: missing field `iam_service` value=private"
        self.assertEqual(classify_startup(output), ["CONFIG_SECTION_MISSING"])
        self.assertEqual(classify_startup(b"unrecognized private text"), ["UNCLASSIFIED"])

    def test_mesh_fixture_reuses_kind_optional_guards_and_verifies_each_service_leaf(self):
        run_id = "a" * 32
        config = yaml.safe_load((ROOT / "ops/tests/k6/ci/config/envoy.yaml").read_text(encoding="utf-8"))
        recovery_mesh.add_companions(config, run_id)
        routes = recovery_mesh.routes(config, "ci_browser")
        leaves = identities(run_id, companion_mesh=True)
        for service in COMPANIONS:
            selected = [item for item in routes if item.get("route", {}).get("cluster") == service]
            self.assertEqual(selected[-1]["match"], {"prefix": f"/{service}/"})
            self.assertNotIn("typed_per_filter_config", selected[-1])
            if service in ("hive", "manifesto"):
                self.assertEqual(len(selected), 2)
                self.assertIn({"name": "authorization", "present_match": False},
                              selected[0]["match"]["headers"])
                self.assertTrue(selected[0]["typed_per_filter_config"]["envoy.filters.http.ext_authz"]["disabled"])
            cluster, = [item for item in config["static_resources"]["clusters"] if item["name"] == service]
            tls = cluster["transport_socket"]["typed_config"]
            san, = tls["common_tls_context"]["validation_context"]["match_typed_subject_alt_names"]
            self.assertEqual(tls["sni"], mesh_hostname(run_id, service))
            self.assertEqual(leaves["mesh-" + service], ["DNS:" + san["matcher"]["exact"]])


if __name__ == "__main__":
    unittest.main()
