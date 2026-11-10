"""Offline Helm checks for native PostgreSQL TLS and missing-only SQL."""
import unittest

from ops.tests import test_minimal_native_openbao as bao


class NativePostgresDeclarationsTests(unittest.TestCase):
    render = bao.NativeOpenBaoDeclarationsTests.render

    def test_postgres_create_is_explicit_tls_pvc_and_credential_file_only(self):
        objects = self.render("postgres.create=true")
        self.assertEqual(len(objects), 4)
        self.assertEqual({obj["kind"] for obj in objects}, {"ConfigMap", "Service", "PersistentVolumeClaim", "Deployment"})
        deployment = next(obj for obj in objects if obj["kind"] == "Deployment")
        self.assertEqual(deployment["metadata"]["name"], "postgres")
        self.assertEqual(deployment["spec"]["strategy"], {"type": "Recreate"})
        self.assertEqual(deployment["spec"]["replicas"], 1)
        pod = deployment["spec"]["template"]["spec"]
        self.assertFalse(pod["automountServiceAccountToken"])
        container = pod["containers"][0]
        self.assertIn("postgres:15.12-alpine3.21@sha256:", container["image"])
        self.assertIn("ssl=on", container["args"])
        self.assertIn("ssl_min_protocol_version=TLSv1.2", container["args"])
        env = {item["name"]: item["value"] for item in container["env"]}
        self.assertEqual(env["POSTGRES_PASSWORD_FILE"], "/run/postgres/password")
        self.assertNotIn("POSTGRES_PASSWORD", env)
        self.assertEqual(env["PGSSLMODE"], "verify-full")
        self.assertEqual(env["PGHOST"], "postgres.aiforall-local-full.svc.cluster.local")
        self.assertEqual(env["PGHOSTADDR"], "127.0.0.1", "readiness must not wait for its own Ready Service endpoint")
        self.assertEqual(env["PGSSLROOTCERT"], "/tls/ca.crt")
        probe = " ".join(container["readinessProbe"]["exec"]["command"])
        self.assertIn("psql", probe)
        self.assertIn("SELECT 1", probe)
        self.assertNotIn("pg_isready", probe)
        config = next(obj for obj in objects if obj["kind"] == "ConfigMap")["data"]["pg_hba.conf"]
        self.assertIn("hostnossl all all 0.0.0.0/0 reject", config)
        self.assertIn("hostnossl all all ::/0 reject", config)
        self.assertIn("hostssl all all 0.0.0.0/0 scram-sha-256", config)
        self.assertNotIn("host all all", config)
        self.assertEqual(next(v for v in pod["volumes"] if v["name"] == "data")["persistentVolumeClaim"]["claimName"], "postgres-data")
        self.assertFalse(any(obj["kind"] == "Secret" for obj in objects))

    def test_sql_initialization_is_independent_native_missing_only_and_tls(self):
        objects = self.render("postgres.initialize=true")
        self.assertEqual({obj["kind"] for obj in objects}, {"ConfigMap", "Job"})
        self.assertFalse(any(obj["metadata"].get("name") in {"postgres", "postgres-data"} for obj in objects))
        job = next(obj for obj in objects if obj["kind"] == "Job")
        self.assertEqual(job["metadata"]["name"], "kind-local-postgres-initialize")
        self.assertEqual(job["spec"]["backoffLimit"], 0)
        container = job["spec"]["template"]["spec"]["containers"][0]
        self.assertIn("psql", " ".join(container["command"]))
        env = {item["name"]: item["value"] for item in container["env"]}
        self.assertEqual(env["PGSSLMODE"], "verify-full")
        self.assertNotIn("PGHOSTADDR", env, "the initializer is a separate Pod, not the database server")
        config = next(obj for obj in objects if obj["kind"] == "ConfigMap")["data"]["initialize.sql"]
        self.assertIn("ON_ERROR_STOP", config)
        self.assertIn("WHERE NOT EXISTS", config)
        self.assertIn("pg_database", config)
        self.assertIn("format('CREATE DATABASE %I'", config)
        self.assertIn("\\gexec", config)
        self.assertNotIn("DROP DATABASE", config)
        self.assertNotIn("TRUNCATE", config)
        self.assertNotIn("PASSWORD", config)

    def test_native_postgres_schema_rejects_sql_injection_and_unpinned_image(self):
        self.render("postgres.initialize=true", "postgres.databases[0]=bad'name", expected=1)
        self.render("postgres.create=true", "postgres.image=postgres:latest", expected=1)


if __name__ == "__main__":
    unittest.main()
