//! Wiremock-backed OpenFGA permission fixtures.
//!
//! The shared [`crate::wiremock::MockServerFixture`] hosts a single in-process
//! `wiremock::MockServer`; this module wraps it with a typed
//! [`OpenFgaMockService`] that mounts stubs at the same
//! `POST /stores/{store_id}/check` endpoint that
//! `rustycog_permission::OpenFgaPermissionChecker` calls.
//!
//! Use [`OpenFgaFixtures`] as the entry point so test code reads as
//! `let openfga = OpenFgaFixtures::service().await;`.
//!
//! Tests that mount these stubs must be marked `#[serial]` because the
//! underlying wiremock server is a process-wide singleton bound to a fixed
//! port.

pub mod resources;
pub mod service;

pub use resources::{CheckRequestBody, CheckResponseBody, CheckTupleKey};
pub use service::OpenFgaMockService;

/// Namespace for OpenFGA permission fixtures.
pub struct OpenFgaFixtures;

impl OpenFgaFixtures {
    /// Build a fake OpenFGA `Check` endpoint with the default store id.
    pub async fn service() -> OpenFgaMockService {
        OpenFgaMockService::new().await
    }

    /// Build a fake OpenFGA `Check` endpoint pinned to `store_id`.
    pub async fn service_with_store_id(store_id: impl Into<String>) -> OpenFgaMockService {
        OpenFgaMockService::with_store_id(store_id).await
    }
}
