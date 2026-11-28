import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from binaries import RECOVERY_ROLE, ROOT, inventory
import inputs
from recovery_monolith import PORT, create_monolith


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
            create_monolith(campaign)

        name, secret = campaign.secret.call_args.args
        deployment, = campaign.create.call_args.args
        self.assertEqual(deployment["metadata"]["name"], name)
        pod = deployment["spec"]["template"]["spec"]
        self.assertFalse(pod["automountServiceAccountToken"])
        config = tomllib.loads(secret["k6.toml"])
        self.assertEqual(config["database"]["db"], values["database"])
        self.assertEqual(config["auth"]["jwt"]["jwks_url"], "http://127.0.0.1:8080/iam/.well-known/jwks.json")
        self.assertEqual(config["auth"]["session_liveness"]["endpoint"],
                         "http://127.0.0.1:8080/iam/internal/auth/session-liveness")
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


if __name__ == "__main__":
    unittest.main()
