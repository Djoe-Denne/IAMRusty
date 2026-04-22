//! Component-catalog wiremock fixtures for Manifesto integration tests.
//!
//! Manifesto's `add_component` use case calls
//! `ComponentServicePort::list_available_components()` every time a new
//! component is added. In production that resolves to a real
//! component-catalog HTTP service; in tests we stub it via this module
//! using the shared singleton wiremock server.
//!
//! Use [`ComponentServiceFixtures`] as the entry point so test code reads
//! as `let catalog = ComponentServiceFixtures::service().await;`.
//!
//! Tests that mount these stubs must be marked `#[serial]` because the
//! underlying wiremock server is a process-wide singleton bound to a fixed
//! port.

pub mod resources;
pub mod service;

pub use resources::ComponentInfoBody;
pub use service::ComponentServiceMockService;

/// Namespace for component-catalog fixtures.
pub struct ComponentServiceFixtures;

impl ComponentServiceFixtures {
    /// Build a fake catalog endpoint with no stubs mounted.
    pub async fn service() -> ComponentServiceMockService {
        ComponentServiceMockService::new().await
    }
}
