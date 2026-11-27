//! One tracked, disposable PG15 cluster; no shared SDK database or external SQL target.

use rustycog::testing::common::{start_service_testcontainer, OwnedTestContainer};
use rustycog::testing::testcontainers::{
    core::ContainerPort, runners::AsyncRunner, GenericImage, ImageExt,
};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use std::{sync::Arc, time::Duration};

const SQL_LIMIT: Duration = Duration::from_secs(8);
const APPLY_LIMIT: Duration = Duration::from_secs(20);

pub struct LogicalPostgres {
    container: OwnedTestContainer,
    password: String,
    port: u16,
    pub writer: Option<Arc<DatabaseConnection>>,
    pub subscriber: Option<Arc<DatabaseConnection>>,
}

impl LogicalPostgres {
    pub async fn start() -> anyhow::Result<Self> {
        anyhow::ensure!(
            std::env::var("RUSTYCOG_TEST_RUNNER_MODE").as_deref() == Ok("local")
                && std::env::var("RUSTYCOG_TEST_RUNNER_HOST")
                    .map_or(true, |host| host == "127.0.0.1" || host == "localhost"),
            "logical-replication fixture requires the local loopback runner"
        );
        let mut config = iam_configuration::load_config_fresh::<iam_configuration::AppConfig>()?;
        let password = uuid::Uuid::new_v4().simple().to_string();
        let name = format!("iam-logical-{}", uuid::Uuid::new_v4().simple());
        // A dedicated typed-config cache key avoids the stock fixture's cached
        // port when this case runs alongside the frozen oracle in the same binary.
        config.database.db.clone_from(&name);
        config.database.creds.username = "replica_fixture".into();
        config.database.port = 0;
        let port = config.database.actual_port();
        let image = GenericImage::new("postgres", "15-alpine")
            .with_env_var("POSTGRES_USER", "replica_fixture")
            .with_env_var("POSTGRES_PASSWORD", &password)
            .with_env_var("POSTGRES_DB", "postgres")
            .with_container_name(&name)
            .with_mapped_port(port, ContainerPort::Tcp(5432))
            .with_startup_timeout(Duration::from_secs(60))
            .with_cmd([
                "postgres",
                "-c",
                "wal_level=logical",
                "-c",
                "max_replication_slots=10",
                "-c",
                "max_wal_senders=10",
            ]);
        let container = start_service_testcontainer(image.start(), "iam-logical", &name, port)
            .await
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            container,
            password,
            port,
            writer: None,
            subscriber: None,
        })
    }

    pub fn writer_url(&self) -> String {
        self.url("iam_writer")
    }

    fn url(&self, database: &str) -> String {
        format!(
            "postgres://replica_fixture:{}@127.0.0.1:{}/{database}",
            self.password, self.port
        )
    }

    async fn connect(&self, database: &str) -> anyhow::Result<Arc<DatabaseConnection>> {
        // The official image's temporary initialization server is socket-only.
        // A successful TCP SQL connection proves the final server is ready.
        let connect = async {
            let mut ticks = tokio::time::interval(Duration::from_millis(100));
            loop {
                ticks.tick().await;
                let mut options = ConnectOptions::new(self.url(database));
                options
                    .max_connections(1)
                    .connect_timeout(SQL_LIMIT)
                    .acquire_timeout(SQL_LIMIT)
                    .sqlx_logging(false);
                // SQL errors and URLs are deliberately not included in evidence.
                if let Ok(Ok(db)) =
                    tokio::time::timeout(SQL_LIMIT, Database::connect(options)).await
                {
                    return db;
                }
            }
        };
        let db = tokio::time::timeout(Duration::from_secs(30), connect)
            .await
            .map_err(|_| anyhow::anyhow!("local PG15 SQL readiness deadline"))?;
        execute(&db, "SET statement_timeout = '7s'").await?;
        Ok(Arc::new(db))
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        let mapped = self
            .container
            .mapped_port(ContainerPort::Tcp(5432))
            .await
            .map_err(anyhow::Error::msg)?;
        anyhow::ensure!(mapped == self.port, "owned PG15 mapped-port mismatch");
        let admin = self.connect("postgres").await?;
        let version = scalar_bool(
            &admin,
            "SELECT current_setting('server_version_num')::int BETWEEN 150000 AND 159999 \
             AND current_setting('wal_level') = 'logical' AS ok",
        )
        .await?;
        anyhow::ensure!(version, "fixture must be PG15 with logical WAL");
        execute(&admin, "CREATE DATABASE iam_writer").await?;
        execute(&admin, "CREATE DATABASE iam_subscriber").await?;
        self.writer = Some(self.connect("iam_writer").await?);
        self.subscriber = Some(self.connect("iam_subscriber").await?);
        self.container.ready().map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub async fn prove_transport(&self) -> anyhow::Result<()> {
        let writer = self.writer.as_ref().expect("initialized writer");
        let subscriber = self.subscriber.as_ref().expect("initialized subscriber");
        for db in [writer, subscriber] {
            execute(
                db,
                "CREATE TABLE public.replica_transport_control (challenge UUID PRIMARY KEY)",
            )
            .await?;
        }
        execute(
            writer,
            "CREATE PUBLICATION iam_control_publication FOR TABLE public.replica_transport_control",
        )
        .await?;
        self.subscribe("iam_control", "iam_control_publication")
            .await?;
        anyhow::ensure!(
            ready(subscriber, "iam_control", 1).await?,
            "control subscription never reached ready"
        );
        let challenge = uuid::Uuid::new_v4();
        execute(
            writer,
            &format!("INSERT INTO public.replica_transport_control VALUES ('{challenge}')"),
        )
        .await?;
        anyhow::ensure!(
            wait_bool(
                subscriber,
                &format!(
                    "SELECT EXISTS (SELECT 1 FROM public.replica_transport_control \
                     WHERE challenge = '{challenge}') AS ok"
                ),
            )
            .await?,
            "control transport failed before exercising production projection"
        );
        eprintln!("PG15 control barrier applied through ordinary logical replication");
        Ok(())
    }

    pub async fn subscribe_projection(&self) -> anyhow::Result<()> {
        let writer = self.writer.as_ref().expect("initialized writer");
        let subscriber = self.subscriber.as_ref().expect("initialized subscriber");
        // Only subscriber DDL and test-owned publication/slots are fixture writes.
        // Source relations and every source epoch must come from production.
        execute(
            subscriber,
            "CREATE TABLE public.iam_session_liveness_epochs \
             (subject UUID PRIMARY KEY, auth_epoch TEXT NOT NULL)",
        )
        .await?;
        execute(
            subscriber,
            "CREATE TABLE public.iam_session_liveness_checkpoints (challenge UUID PRIMARY KEY)",
        )
        .await?;
        execute(
            writer,
            "CREATE PUBLICATION iam_projection_publication FOR TABLE \
             public.iam_session_liveness_epochs, public.iam_session_liveness_checkpoints",
        )
        .await?;
        self.subscribe("iam_projection", "iam_projection_publication")
            .await?;
        anyhow::ensure!(
            ready(subscriber, "iam_projection", 2).await?,
            "both production relations must complete initial synchronization"
        );
        Ok(())
    }

    async fn subscribe(&self, name: &str, publication: &str) -> anyhow::Result<()> {
        let writer = self.writer.as_ref().expect("initialized writer");
        let subscriber = self.subscriber.as_ref().expect("initialized subscriber");
        // PG15 same-cluster restriction: create the pgoutput slot separately.
        // Neither operation is wrapped in a transaction.
        query(
            writer,
            &format!("SELECT * FROM pg_create_logical_replication_slot('{name}', 'pgoutput')"),
        )
        .await?;
        // This connection runs INSIDE the owned cluster, never via the host's port.
        // Never log this SQL: the disposable connection credential is embedded.
        execute(
            subscriber,
            &format!(
                "CREATE SUBSCRIPTION {name} CONNECTION \
                 'host=127.0.0.1 port=5432 user=replica_fixture password={} \
                 dbname=iam_writer connect_timeout=5' PUBLICATION {publication} \
                 WITH (create_slot=false, slot_name='{name}', copy_data=true)",
                self.password
            ),
        )
        .await
    }

    pub async fn cleanup(self) -> anyhow::Result<()> {
        let listeners = if let Some(writer) = &self.writer {
            crate::common::cleanup_writer_test_servers(writer).await
        } else {
            Ok(())
        };
        // The whole cluster is disposable. Stop by the tracked owned ID even
        // after SQL/setup/assertion failure; no DROP SUBSCRIPTION or force cleanup.
        let stopped = self.container.stop().await.map_err(anyhow::Error::msg);
        let removed = if stopped.is_ok() {
            self.container.rm().await.map_err(anyhow::Error::msg)
        } else {
            Ok(())
        };
        listeners.and(stopped).and(removed)
    }
}

pub async fn execute(db: &DatabaseConnection, sql: &str) -> anyhow::Result<()> {
    tokio::time::timeout(
        SQL_LIMIT,
        db.execute(Statement::from_string(DatabaseBackend::Postgres, sql)),
    )
    .await
    .map_err(|_| anyhow::anyhow!("fixture SQL execution deadline"))?
    .map_err(|_| anyhow::anyhow!("fixture SQL execution failed (details redacted)"))?;
    Ok(())
}

async fn query(db: &DatabaseConnection, sql: &str) -> anyhow::Result<sea_orm::QueryResult> {
    tokio::time::timeout(
        SQL_LIMIT,
        db.query_one(Statement::from_string(DatabaseBackend::Postgres, sql)),
    )
    .await
    .map_err(|_| anyhow::anyhow!("fixture SQL query deadline"))?
    .map_err(|_| anyhow::anyhow!("fixture SQL query failed (details redacted)"))?
    .ok_or_else(|| anyhow::anyhow!("fixture SQL query returned no row"))
}

pub async fn scalar_bool(db: &DatabaseConnection, sql: &str) -> anyhow::Result<bool> {
    query(db, sql)
        .await?
        .try_get("", "ok")
        .map_err(|_| anyhow::anyhow!("fixture boolean result missing"))
}

async fn wait_bool(db: &DatabaseConnection, sql: &str) -> anyhow::Result<bool> {
    let wait = async {
        let mut ticks = tokio::time::interval(Duration::from_millis(50));
        loop {
            ticks.tick().await;
            if scalar_bool(db, sql).await? {
                return Ok(true);
            }
        }
    };
    tokio::time::timeout(APPLY_LIMIT, wait)
        .await
        .unwrap_or(Ok(false))
}

async fn ready(db: &DatabaseConnection, name: &str, tables: usize) -> anyhow::Result<bool> {
    wait_bool(
        db,
        &format!(
            "SELECT COUNT(*) = {tables} AND COALESCE(bool_and(r.srsubstate = 'r'), false) AS ok \
             FROM pg_subscription_rel r JOIN pg_subscription s ON s.oid = r.srsubid \
             WHERE s.subname = '{name}'"
        ),
    )
    .await
}

pub async fn epoch(db: &DatabaseConnection, subject: uuid::Uuid) -> anyhow::Result<Option<String>> {
    let row = query(
        db,
        &format!(
            "SELECT (SELECT auth_epoch FROM public.iam_session_liveness_epochs \
             WHERE subject = '{subject}') AS epoch"
        ),
    )
    .await?;
    row.try_get("", "epoch")
        .map_err(|_| anyhow::anyhow!("projection epoch result invalid"))
}

pub async fn wait_epoch(db: &DatabaseConnection, subject: uuid::Uuid) -> anyhow::Result<bool> {
    wait_bool(
        db,
        &format!(
            "SELECT EXISTS (SELECT 1 FROM public.iam_session_liveness_epochs \
             WHERE subject = '{subject}' AND auth_epoch = '1') AS ok"
        ),
    )
    .await
}
