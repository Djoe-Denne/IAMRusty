//! Common test utilities for Hive
//!
//! Mirrors rustycog-testing patterns and Telegraph tests structure.

use async_trait::async_trait;
use reqwest::Client;
use rustycog_config::ServerConfig;
use rustycog_testing::*;
use std::sync::Arc;

use hive_configuration::AppConfig;
use hive_setup::app::AppBuilder;
use hive_migration::{Migrator, MigratorTrait};
use anyhow::anyhow;

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

/// Setup Hive test server and return (fixture, base_url, client)
pub async fn setup_test_server(
) -> Result<(HiveTestFixture, String, Client), Box<dyn std::error::Error>> {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor.clone()).await?;

    let (server_url, client) = rustycog_testing::setup_test_server::<HiveTestDescriptor, HiveTestFixture>(descriptor).await?;

    Ok((fixture, server_url, client))
}


