//! Common test utilities for Hive
//!
//! Mirrors rustycog-testing patterns and Telegraph tests structure, plus
//! the wiremock-backed OpenFGA fake every permission-touching Hive test
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

// Re-export the OpenFGA wiremock fake so tests can arrange `Check`
// decisions without pulling `rustycog_testing::permission` paths into
// every file. The harness mounts a permissive default so happy-path tests
// pass without per-test arrangement; denial tests `reset()` and mount the
// per-tuple deny they care about.
pub use rustycog_testing::permission::{OpenFgaFixtures, OpenFgaMockService};

// Re-export the permission domain types tests need to express stub
// tuples for denial scenarios.
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
}

/// Bootstrap the Hive test server **and** the OpenFGA wiremock fake.
///
/// Returns a 4-tuple:
/// 1. [`HiveTestFixture`] — owns the test DB and migration lifecycle.
/// 2. `String` — base URL of the live HTTP server.
/// 3. `Client` — `reqwest` client preconfigured for the test server.
/// 4. [`OpenFgaMockService`] — wiremock fake of OpenFGA's `Check`
///    endpoint, pre-arranged with `mock_check_any(true)` so every
///    permission-gated route passes the route guard by default. Tests
///    that assert a `403` reset the fake and mount per-tuple deny stubs.
///
/// Both this fake and `Manifesto`'s share the singleton wiremock listener
/// at `127.0.0.1:3000`, so tests must remain `#[serial]`.
pub async fn setup_test_server(
) -> Result<(HiveTestFixture, String, Client, OpenFgaMockService), Box<dyn std::error::Error>> {
    // Bring up the OpenFGA wiremock fake **before** booting the app so the
    // production `OpenFgaPermissionChecker` constructed in `AppBuilder`
    // resolves `http://127.0.0.1:3000/stores/.../check` against a live
    // mock server. `MockServerFixture::new()` (called inside `service()`)
    // eagerly resets every previously mounted stub for test isolation.
    let openfga = OpenFgaFixtures::service().await;

    // Permissive default for the route-guard `Check` calls. Mounted here
    // so every permission-gated route in the suite passes the OpenFGA
    // guard. Denial tests call `openfga.reset().await` then mount their
    // per-tuple deny — wiremock matches in registration order so the
    // catch-all must be wiped first.
    openfga.mock_check_any(true).await;

    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor.clone()).await?;

    let (server_url, client) = rustycog_testing::setup_test_server::<HiveTestDescriptor, HiveTestFixture>(descriptor).await?;

    Ok((fixture, server_url, client, openfga))
}


