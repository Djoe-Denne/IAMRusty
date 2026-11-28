import sys
import unittest
from pathlib import Path
from unittest.mock import Mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from binaries import Refused
from session_replica import SessionReplica


class SessionReplicaTests(unittest.TestCase):
    def setUp(self):
        self.run = "a" * 32
        self.writer = "k6_" + self.run
        self.replica = SessionReplica(self.run, self.writer)

    def test_rejects_a_database_outside_the_owned_run(self):
        for run, database in [(self.run, "postgres"), ("a" * 31, self.writer)]:
            with self.subTest(run=run, database=database), self.assertRaises(Refused):
                SessionReplica(run, database)

    def test_refuses_nonlogical_server_before_modifying_replication(self):
        sql = Mock(return_value="replica")
        with self.assertRaises(Refused):
            self.replica.subscribe(sql)
        sql.assert_called_once_with("SHOW wal_level;", self.writer)

    def test_creates_slot_on_writer_before_subscription_on_read_copy(self):
        statements = []

        def sql(statement, database):
            statements.append((statement, database))
            return "logical"

        self.replica.subscribe(sql)
        wal, publication, tables, slot, subscription, readiness = statements
        self.assertEqual(wal, ("SHOW wal_level;", self.writer))
        self.assertIn("CREATE PUBLICATION", publication[0])
        self.assertEqual(publication[1], self.writer)
        self.assertIn("GRANT SELECT", tables[0])
        self.assertEqual(tables[1], self.replica.subscriber)
        self.assertIn("pg_create_logical_replication_slot", slot[0])
        self.assertEqual(slot[1], self.writer)
        self.assertIn("create_slot = false, copy_data = true", subscription[0])
        self.assertIn("sslmode=verify-full", subscription[0])
        self.assertEqual(subscription[1], self.replica.subscriber)
        self.assertIn(self.replica.generation, readiness[0])
        self.assertEqual(readiness[1], self.replica.subscriber)

    def test_restricts_both_sql_logins_to_their_own_databases_and_tls(self):
        original = "hostssl all all 0.0.0.0/0 scram-sha-256\n"
        config = {"kind": "ConfigMap", "metadata": {"name": "kind-local-postgres-tls"},
                  "data": {"pg_hba.conf": original}}
        self.replica.postgres_options(config)
        rules = config["data"]["pg_hba.conf"].splitlines()
        for cidr in ("0.0.0.0/0", "::/0"):
            for database, role in ((self.writer, "iam_session_replication"),
                                   (self.replica.subscriber, "iam_session_admission")):
                permitted = f"hostssl {database} {role} {cidr} scram-sha-256"
                refused = f"host all {role} {cidr} reject"
                self.assertLess(rules.index(permitted), rules.index(refused))
        self.assertEqual(rules[-1], original.strip())

    def test_gateway_gets_only_subscriber_and_scoped_http_capabilities(self):
        gateway = self.replica.gateway_secret(b"fixture-identity")
        dsn = gateway["EXT_AUTHZ_REPLICA_DSN"]
        self.assertIn("iam_session_admission:", dsn)
        self.assertIn(f"/{self.replica.subscriber}?", dsn)
        self.assertIn("sslmode=verify-full", dsn)
        self.assertNotIn(self.replica.replication_password, dsn)
        self.assertEqual(gateway["EXT_AUTHZ_CHECKPOINT_TOKEN"],
                         self.replica.iam_secret()["K6_IAM_CHECKPOINT_TOKEN"])
        self.assertNotEqual(gateway["EXT_AUTHZ_CHECKPOINT_TOKEN"],
                            gateway["K6_EXT_AUTHZ_LIVENESS_TOKEN"])

    def test_bootstrap_witness_is_scoped_to_subscriber_generation(self):
        sql = Mock(side_effect=["f", "t"])
        self.assertFalse(self.replica.observed_marker(sql))
        self.assertTrue(self.replica.observed_marker(sql))
        statement, database = sql.call_args.args
        self.assertEqual(database, self.replica.subscriber)
        self.assertIn(self.replica.generation, statement)
        self.assertIn("iam_session_liveness_checkpoints", statement)


if __name__ == "__main__":
    unittest.main()
