import sys
import tempfile
import unittest
import base64
import hmac
import json
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from binaries import RECOVERY_ROLE, ROOT, inventory
import inputs
from recovery_monolith import PORT, classify_startup, create_monolith, snapshot_fixture


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
        self.assertFalse(any(entry.get("value") == secret["K6_MONOLITH_LIVENESS"] for entry in main["env"]))
        migration_names = set(inventory(ROOT, RECOVERY_ROLE).values()) - {"oodhive-monolith"}
        self.assertEqual({entry["command"][0].split("/")[-1] for entry in pod["initContainers"]}, migration_names)
        for migration in pod["initContainers"]:
            service = migration["name"]
            database = values["database"] + "_" + service
            self.assertIn(f"/{database}?sslmode=verify-full", secret[service + "-database-url"])
            self.assertIn({"name": service.upper() + "_DATABASE__DB", "value": database}, main["env"])
        campaign.forward.assert_called_once_with("monolith", PORT, 8080)

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


if __name__ == "__main__":
    unittest.main()
