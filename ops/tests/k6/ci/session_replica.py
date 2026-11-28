"""Database bootstrap for session admission on the owned hosted Kind runner."""

import re
import secrets
import uuid

from binaries import ROOT, require
from runtime import NS

SQL = ROOT / "ops/deploy/iam-session-replica"
POSTGRES_HOST = f"postgres.{NS}.svc.cluster.local"
IAM_URL = f"https://iam.{NS}.svc.cluster.local:8443/iam/internal/auth/session-liveness"


class SessionReplica:
    def __init__(self, run_id, writer):
        require(re.fullmatch(r"[0-9a-f]{32}", run_id) and writer == f"k6_{run_id}",
                "E2E_REPLICA_DATABASE_SCOPE")
        self.writer = writer
        self.subscriber = writer + "_admission"
        self.generation = str(uuid.uuid4())
        self.replication_password = secrets.token_hex(32)
        self.read_password = secrets.token_hex(32)
        self.checkpoint_token = secrets.token_hex(32)
        self.liveness_token = secrets.token_hex(32)
        require(self.checkpoint_token != self.liveness_token, "E2E_REPLICA_CAPABILITIES")

    def postgres_options(self, item):
        """Tighten only rendered resources, before creating them in this run."""
        name = item["metadata"]["name"]
        if name == "postgres" and item["kind"] == "Deployment":
            containers = item["spec"]["template"]["spec"]["containers"]
            postgres, = [container for container in containers if container["name"] == "postgres"]
            postgres["args"].extend(["-c", "wal_level=logical"])
        elif name == "kind-local-postgres-tls" and item["kind"] == "ConfigMap":
            allow = (
                (self.subscriber, "iam_session_admission"),
                (self.writer, "iam_session_replication"),
            )
            restrictions = []
            for ip in ("0.0.0.0/0", "::/0"):
                for database, role in allow:
                    restrictions += [f"hostssl {database} {role} {ip} scram-sha-256",
                                     f"host all {role} {ip} reject"]
            item["data"]["pg_hba.conf"] = "\n".join(restrictions) + "\n" + item["data"]["pg_hba.conf"]

    def create_database(self, sql):
        sql(f'CREATE DATABASE "{self.subscriber}";', "postgres")
        sql(
            f"CREATE ROLE iam_session_replication LOGIN REPLICATION PASSWORD '{self.replication_password}';\n"
            f"CREATE ROLE iam_session_admission LOGIN PASSWORD '{self.read_password}';",
            "postgres",
        )

    def iam_secret(self):
        return {"K6_IAM_CHECKPOINT_TOKEN": self.checkpoint_token}

    def gateway_secret(self, identity):
        return {
            "workload.pem": identity,
            "EXT_AUTHZ_REPLICA_DSN":
                f"postgresql://iam_session_admission:{self.read_password}@{POSTGRES_HOST}:5432/"
                f"{self.subscriber}?sslmode=verify-full&sslrootcert=/app/certs/ca.crt",
            "EXT_AUTHZ_REPLICA_GENERATION": self.generation,
            "EXT_AUTHZ_CHECKPOINT_URL": IAM_URL + "/checkpoint",
            "EXT_AUTHZ_CHECKPOINT_TOKEN": self.checkpoint_token,
            "EXT_AUTHZ_LIVENESS_URL": IAM_URL,
            "K6_EXT_AUTHZ_LIVENESS_TOKEN": self.liveness_token,
        }

    def subscribe(self, sql):
        require(sql("SHOW wal_level;", self.writer) == "logical", "E2E_REPLICA_WAL")
        sql((SQL / "publisher.sql").read_text(encoding="utf-8"), self.writer)
        sql((SQL / "subscriber.sql").read_text(encoding="utf-8"), self.subscriber)
        # PG15 requires separately creating the slot for same-cluster apply.
        sql("SELECT pg_create_logical_replication_slot('iam_projection', 'pgoutput');", self.writer)
        connection = (
            f"host={POSTGRES_HOST} hostaddr=127.0.0.1 port=5432 dbname={self.writer} "
            f"user=iam_session_replication password={self.replication_password} "
            "sslmode=verify-full sslrootcert=/tls/ca.crt connect_timeout=5"
        )
        sql(
            "CREATE SUBSCRIPTION iam_projection "
            f"CONNECTION '{connection}' PUBLICATION iam_projection_publication "
            "WITH (create_slot = false, copy_data = true);",
            self.subscriber,
        )
        readiness = (SQL / "readiness.sql").read_text(encoding="utf-8")
        sql(readiness.replace(":'generation'", f"'{self.generation}'"), self.subscriber)

    def observed_marker(self, sql):
        """Only a boolean crosses the SQL witness boundary."""
        return sql(
            "SELECT EXISTS (SELECT 1 FROM public.iam_session_liveness_ready "
            f"WHERE configured_generation = '{self.generation}') AND EXISTS "
            "(SELECT 1 FROM public.iam_session_liveness_checkpoints);",
            self.subscriber,
        ) == "t"
