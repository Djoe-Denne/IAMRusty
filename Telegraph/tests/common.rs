//! Common test utilities for Telegraph
//!
//! Provides test infrastructure following rustycog-testing patterns
//! and Telegraph-specific test setup, including the real OpenFGA
//! testcontainer every permission-gated route is routed through
//! (mirrors `Manifesto/tests/common.rs`).

use async_trait::async_trait;
use reqwest::Client;
use rustycog_config::ServerConfig;
use rustycog_testing::sqs_testcontainer::TestSqs;
use rustycog_testing::*;
use std::sync::Arc;
use std::sync::OnceLock;
use telegraph_configuration::{load_config, setup_logging, TelegraphConfig};
use telegraph_http_server::SERVICE_PREFIX;
use telegraph_setup::app::{AppBuilder, TelegraphApp};
use telegraphmigration::{Migrator, MigratorTrait};

// Re-export the real OpenFGA testcontainer fixture so tests can arrange
// `Check` decisions by writing real relationship tuples without pulling
// `rustycog_testing::common::openfga_testcontainer` paths into every file.
// The harness writes **no** permissive default; each test must
// explicitly call `openfga.allow(subject, action, resource)` for every
// tuple the route guard will check (default = deny).
pub use rustycog_testing::common::openfga_testcontainer::TestOpenFga;

// Re-export the permission domain types tests need to express tuples.
pub use rustycog_permission::{Permission, ResourceRef, Subject};

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::smtp::testcontainer::TestSmtp;
use fixtures::*;

static mut APP: Option<TelegraphApp> = None;
/// Telegraph test descriptor following rustycog-testing patterns
pub struct TelegraphTestDescriptor;

#[async_trait]
impl ServiceTestDescriptor<TelegraphTestFixture> for TelegraphTestDescriptor {
    type Config = TelegraphConfig;

    async fn build_app(
        &self,
        config: TelegraphConfig,
        server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        let app = AppBuilder::new(config).build().await?;
        unsafe {
            APP.replace(app);
        }
        Ok(())
    }

    async fn run_app(
        &self,
        config: TelegraphConfig,
        server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        unsafe {
            APP.as_ref().unwrap().run(server_config).await?;
        }
        Ok(())
    }

    async fn run_migrations_up(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::down(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        true
    }

    fn has_openfga(&self) -> bool {
        true
    }
}

impl TelegraphTestDescriptor {
    fn has_smtp(&self) -> bool {
        true
    }
}

/// Telegraph-specific test fixture with SMTP capabilities
pub struct TelegraphTestFixture {
    pub fixture: TestFixture,
    pub smtp: Option<std::sync::Arc<TestSmtp>>,
}

impl TelegraphTestFixture {
    /// Create a new Telegraph test fixture with optional SMTP
    pub async fn new(
        descriptor: Arc<TelegraphTestDescriptor>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = TestFixture::new(descriptor).await?;

        // Initialize SMTP container if needed
        let smtp = if TelegraphTestDescriptor.has_smtp() {
            Some(TestSmtp::new().await?)
        } else {
            None
        };

        Ok(Self { fixture, smtp })
    }

    /// Get the database connection
    pub fn db(&self) -> Arc<sea_orm::DatabaseConnection> {
        self.fixture.db()
    }

    /// Get the SQS client
    pub fn sqs(&self) -> &TestSqs {
        self.fixture.sqs()
    }

    /// Get the SMTP container
    pub fn smtp(&self) -> &std::sync::Arc<TestSmtp> {
        self.smtp.as_ref().expect("SMTP container not initialized")
    }

    /// Get the OpenFGA fixture
    pub fn openfga(&self) -> &TestOpenFga {
        self.fixture.openfga()
    }
}

/// Setup Telegraph test server with database, SMTP, and the real
/// OpenFGA testcontainer.
///
/// Returns a 4-tuple:
/// 1. [`TelegraphTestFixture`] — owns the test DB, SQS testcontainer,
///    SMTP container, and the singleton OpenFGA testcontainer.
/// 2. `String` — base URL of the live HTTP server.
/// 3. `Client` — `reqwest` client preconfigured for the test server.
/// 4. `TestOpenFga` (clone) — typed handle exposing `allow` / `deny`
///    against the real OpenFGA Check pipeline. The harness writes
///    **no** permissive default; each test must explicitly call
///    `openfga.allow(...)` for every tuple the route guard will check
///    (default = deny).
///
/// The OpenFGA fixture is process-global, so tests must remain
/// `#[serial]` to avoid tuple-state collisions.
pub async fn setup_test_server(
) -> Result<(TelegraphTestFixture, String, Client, TestOpenFga), Box<dyn std::error::Error>> {
    // Bring up the OpenFGA testcontainer + database first so the env
    // vars are populated before the app boots.
    let descriptor = Arc::new(TelegraphTestDescriptor);
    let fixture = TelegraphTestFixture::new(descriptor.clone()).await?;
    let openfga = fixture.openfga().clone();
    fixture
        .smtp()
        .clear_emails()
        .await
        .expect("Failed to clear emails");
    // Start the Telegraph server
    let (server_url, client) = rustycog_testing::setup_test_server::<
        TelegraphTestDescriptor,
        TelegraphTestFixture,
    >(descriptor)
    .await?;

    Ok((fixture, prefixed_url(server_url), client, openfga))
}

fn prefixed_url(server_url: String) -> String {
    format!("{server_url}{SERVICE_PREFIX}")
}
