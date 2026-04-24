//! Common test utilities for Manifesto
//!
//! Provides test infrastructure following rustycog-testing patterns
//! and Manifesto-specific test setup, including the real OpenFGA
//! testcontainer every permission-touching test routes through.

// Test utilities from rustycog-testing
pub use rustycog_testing::TestFixture;
pub use rustycog_testing::*;

// Re-export the real OpenFGA testcontainer fixture so tests can arrange
// `Check` decisions by writing real relationship tuples without pulling
// `rustycog_testing::common::openfga_testcontainer` paths into every file.
pub use rustycog_testing::common::openfga_testcontainer::TestOpenFga;

// Re-export the permission domain types tests need to express tuples.
pub use rustycog_permission::{Permission, ResourceRef, Subject};

// Re-export the component-catalog fixture so tests that arrange a custom
// catalog (or an error scenario) don't have to reach into
// `tests/fixtures/component_service/`.
#[path = "fixtures/component_service/mod.rs"]
mod component_service_fixture;
pub use component_service_fixture::{
    ComponentInfoBody, ComponentServiceFixtures, ComponentServiceMockService,
};

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

    fn has_openfga(&self) -> bool {
        true
    }
}

/// Bootstrap the Manifesto test server plus the real OpenFGA
/// testcontainer and the wiremock-backed component-catalog fake every
/// component-gated route depends on.
///
/// Returns a 5-tuple:
/// 1. `TestFixture` — owns the test DB, the singleton `openfga/openfga`
///    testcontainer, and the migration lifecycle.
/// 2. `String` — base URL of the live HTTP server.
/// 3. `Client` — `reqwest` client preconfigured for the test server.
/// 4. `TestOpenFga` (clone) — typed handle exposing `allow` / `deny` /
///    `read_tuples` against the real OpenFGA Check pipeline. The harness
///    writes **no** permissive default — each test must explicitly grant
///    every tuple the route guard will check by calling
///    `openfga.allow(subject, action, resource)`. Denial tests simply
///    omit the grant (default = deny).
/// 5. [`ComponentServiceMockService`] — wiremock fake of the upstream
///    component-catalog HTTP service, pre-arranged with the default
///    catalog (`taskboard` + `wiki`).
///
/// The OpenFGA fixture is process-global, so tests must remain
/// `#[serial]` to avoid tuple-state collisions. The component-catalog
/// fake shares the singleton wiremock listener at `127.0.0.1:3000` —
/// `reset()` on it wipes every wiremock stub, including any other
/// fixture mounted on the same singleton.
///
/// Bring-up order: the OpenFGA testcontainer publishes
/// `MANIFESTO_OPENFGA__*` env vars during `TestFixture::new`; the app
/// boots **after** that so its `OpenFgaPermissionChecker` resolves the
/// fixture's URL / store id / model id instead of the `test.toml`
/// placeholders.
pub async fn setup_test_server() -> Result<
    (
        TestFixture,
        String,
        Client,
        TestOpenFga,
        ComponentServiceMockService,
    ),
    Box<dyn std::error::Error>,
> {
    // Bring up the OpenFGA testcontainer + database first so the env
    // vars are populated before the app boots.
    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let openfga = fixture.openfga().clone();

    // Build the wiremock catalog fake **after** OpenFGA so the singleton
    // wiremock reset performed by `MockServerFixture::new()` does not
    // wipe an unrelated fixture.
    let components = ComponentServiceFixtures::service().await;
    components.mock_default_catalog().await;

    let (server_url, client) =
        rustycog_testing::setup_test_server::<ManifestoTestDescriptor, TestFixture>(descriptor)
            .await?;
    Ok((fixture, server_url, client, openfga, components))
}
