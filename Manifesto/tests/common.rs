//! Common test utilities for Manifesto
//!
//! Provides test infrastructure following rustycog-testing patterns
//! and Manifesto-specific test setup

// Test utilities from rustycog-testing
pub use rustycog_testing::TestFixture;
pub use rustycog_testing::*;

// Migration crate import
use manifesto_migration::{Migrator, MigratorTrait};

// Manifesto imports
use async_trait::async_trait;
use manifesto_configuration::ManifestoConfig;
use manifesto_setup::build_and_run;
use reqwest::Client;
use rustycog_config::ServerConfig;
use std::sync::Arc;

pub struct ManifestoTestDescriptor;

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for ManifestoTestDescriptor {
    type Config = ManifestoConfig;

    async fn build_app(
        &self,
        _config: ManifestoConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, config: ManifestoConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        build_and_run(config, server_config, None).await
    }

    async fn run_migrations_up(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        Migrator::down(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        false
    }
}

pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
{
    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) =
        rustycog_testing::setup_test_server::<ManifestoTestDescriptor, TestFixture>(descriptor)
            .await?;
    Ok((fixture, server_url, client))
}


