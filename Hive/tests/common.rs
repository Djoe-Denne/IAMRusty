//! Common test utilities for Hive
//!
//! Mirrors rustycog-testing patterns and Telegraph tests structure, plus
//! the real OpenFGA testcontainer every permission-touching Hive test
//! routes through (mirrors `Manifesto/tests/common.rs`).

use async_trait::async_trait;
use reqwest::Client;
use rustycog_config::ServerConfig;
use rustycog_testing::*;
use std::sync::Arc;

use hive_configuration::AppConfig;
use hive_setup::app::AppBuilder;
use hive_migration::{Migrator, MigratorTrait};
use anyhow::anyhow;

// Re-export the real OpenFGA testcontainer fixture so tests can arrange
// `Check` decisions by writing real relationship tuples without pulling
// `rustycog_testing::common::openfga_testcontainer` paths into every file.
// The harness writes **no** permissive default; each test must
// explicitly call `openfga.allow(subject, action, resource)` for every
// tuple the route guard will check (default = deny).
pub use rustycog_testing::common::openfga_testcontainer::TestOpenFga;

// Re-export the permission domain types tests need to express tuples.
pub use rustycog_permission::{Permission, ResourceRef, Subject};

// Re-export fixtures
#[path = "fixtures/mod.rs"]
pub mod fixtures;

static mut APP: Option<hive_setup::app::Application> = None;

/// Hive test descriptor following rustycog-testing patterns
pub struct HiveTestDescriptor;

#[async_trait]
impl ServiceTestDescriptor<HiveTestFixture> for HiveTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        let app = AppBuilder::new(config).build().await?;
        unsafe {
            APP.replace(app);
        }
        Ok(())
    }

    async fn run_app(
        &self,
        _config: AppConfig,
        server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        // Move application out of the static store to run it (run consumes self)
        let app = unsafe { APP.take() }.ok_or_else(|| anyhow!("App not built"))?;
        app.run(server_config).await?;
        Ok(())
    }

    async fn run_migrations_up(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        println!("Running migrations up");
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        println!("Running migrations down");
        Migrator::down(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        // Hive tests default to queue disabled (NoOp)
        false
    }

    fn has_openfga(&self) -> bool {
        true
    }
}

/// Hive-specific test fixture
pub struct HiveTestFixture {
    pub fixture: rustycog_testing::common::TestFixture,
}

impl HiveTestFixture {
    pub async fn new(
        descriptor: Arc<HiveTestDescriptor>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = rustycog_testing::common::TestFixture::new(descriptor).await?;
        Ok(Self { fixture })
    }

    /// Get the database connection
    pub fn db(&self) -> Arc<sea_orm::DatabaseConnection> {
        self.fixture.db()
    }

    /// Get the OpenFGA fixture
    pub fn openfga(&self) -> &TestOpenFga {
        self.fixture.openfga()
    }
}

/// Bootstrap the Hive test server **and** the real OpenFGA testcontainer.
///
/// Returns a 4-tuple:
/// 1. [`HiveTestFixture`] — owns the test DB, the singleton OpenFGA
///    testcontainer, and the migration lifecycle.
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
) -> Result<(HiveTestFixture, String, Client, TestOpenFga), Box<dyn std::error::Error>> {
    // Bring up the OpenFGA testcontainer + database first so the env
    // vars are populated before the app boots.
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor.clone()).await?;
    let openfga = fixture.openfga().clone();

    let (server_url, client) = rustycog_testing::setup_test_server::<HiveTestDescriptor, HiveTestFixture>(descriptor).await?;

    Ok((fixture, server_url, client, openfga))
}
