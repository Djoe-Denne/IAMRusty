//! Common test utilities for Manifesto
//!
//! Provides test infrastructure following rustycog-testing patterns
//! and Manifesto-specific test setup, including the wiremock-backed
//! OpenFGA fake every permission-touching test routes through.

// Test utilities from rustycog-testing
pub use rustycog_testing::TestFixture;
pub use rustycog_testing::*;

// Re-export the OpenFGA fixture so tests can arrange `Check` decisions
// without pulling rustycog_testing::permission paths into every file.
pub use rustycog_testing::permission::{OpenFgaFixtures, OpenFgaMockService};

// Re-export the permission domain types tests need to express stub tuples.
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
}

/// Bootstrap the Manifesto test server plus the wiremock-backed fakes
/// every permission- or component-gated route depends on.
///
/// Returns a 5-tuple:
/// 1. `TestFixture` — owns the test DB and migration lifecycle.
/// 2. `String` — base URL of the live HTTP server.
/// 3. `Client` — `reqwest` client preconfigured for the test server.
/// 4. [`OpenFgaMockService`] — wiremock fake of OpenFGA's `Check`
///    endpoint, pre-arranged with `mock_check_any(true)` so every
///    permission-gated route passes the route guard by default. Tests
///    that assert a `403` reset the fake and mount per-tuple deny stubs
///    (see `component_api_tests.rs` for the pattern).
/// 5. [`ComponentServiceMockService`] — wiremock fake of the upstream
///    component-catalog HTTP service, pre-arranged with the default
///    catalog (`taskboard` + `wiki`). `add_component` calls
///    `ComponentServicePort::list_available_components()` against this
///    every time it runs.
///
/// Both fakes share the singleton wiremock listener at `127.0.0.1:3000`,
/// so tests must remain `#[serial]`. Calling `reset()` on either handle
/// wipes the **entire** singleton — including stubs mounted by the other
/// fake — so tests that need to remount one will usually want to remount
/// the other too.
///
/// To author a test that asserts the OpenFGA route guard itself denies a
/// specific tuple, reset the OpenFGA handle and mount only the per-tuple
/// deny: wiremock matches stubs in registration order (first-match wins),
/// so a deny mounted on top of the catch-all would never fire.
pub async fn setup_test_server() -> Result<
    (
        TestFixture,
        String,
        Client,
        OpenFgaMockService,
        ComponentServiceMockService,
    ),
    Box<dyn std::error::Error>,
> {
    // Construct **both** wiremock-backed fakes before mounting any stubs.
    // `MockServerFixture::new()` calls `reset_all_mocks()` eagerly, so a
    // sibling fake constructed *after* a stub was mounted would wipe it.
    // Build them back-to-back, then arrange. The second `new()` triggers
    // one redundant reset of the (still empty) singleton, which is a no-op.
    let openfga = OpenFgaFixtures::service().await;
    let components = ComponentServiceFixtures::service().await;

    // Permissive default for the route-guard `Check` calls. Every
    // permission-gated route in the suite passes the OpenFGA guard and
    // execution falls through to domain-level authorization inside the
    // use cases.
    openfga.mock_check_any(true).await;

    // Default catalog for `add_component`'s `GET /api/components` call.
    // `taskboard` and `wiki` cover every component type the checked-in
    // tests request; extend `mock_default_catalog` when adding more.
    components.mock_default_catalog().await;

    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) =
        rustycog_testing::setup_test_server::<ManifestoTestDescriptor, TestFixture>(descriptor)
            .await?;
    Ok((fixture, server_url, client, openfga, components))
}

