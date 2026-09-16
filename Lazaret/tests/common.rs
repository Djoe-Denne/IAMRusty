//! Common test utilities for Lazaret.
//!
//! Prefixed base URL (`…/lazaret`). `has_openfga() == false` — deny-all checker.

use async_trait::async_trait;
use lazaret_configuration::AppConfig;
use lazaret_http::SERVICE_PREFIX;
use lazaret_migration::{Migrator, MigratorTrait};
use lazaret_setup::app::AppBuilder;
use reqwest::Client;
use rustycog::config::ServerConfig;
use rustycog::testing::*;
use std::sync::Arc;

pub use rustycog::testing::TestFixture;

#[allow(unused_imports)]
pub use rustycog::testing::http::jwt::create_jwt_token;

/// Lazaret test descriptor (Postgres only, no OpenFGA container).
pub struct LazaretTestDescriptor;

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for LazaretTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        let app = AppBuilder::new(config).build().await?;
        app.run(server_config).await
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
        false
    }

    fn has_openfga(&self) -> bool {
        false
    }
}

/// Bootstrap the Lazaret test server. Base URL already includes `/lazaret`.
///
/// # Errors
///
/// Returns an error if the fixture or HTTP server cannot start.
pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
{
    let descriptor = Arc::new(LazaretTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) =
        rustycog::testing::setup_test_server::<LazaretTestDescriptor, TestFixture>(descriptor)
            .await?;
    Ok((fixture, prefixed_url(&server_url), client))
}

fn prefixed_url(server_url: &str) -> String {
    format!("{server_url}{SERVICE_PREFIX}")
}
