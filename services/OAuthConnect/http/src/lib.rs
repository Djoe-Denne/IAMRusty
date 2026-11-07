//! Immutable instance routing at the existing v2 HMAC boundary; T0 refuses setup.

use std::sync::Arc;

use axum::Router;
pub use idp_connect_contract::server::ConnectState;
pub use readiness::ReadinessProbe;
pub use rustycog::core::error::DomainError;
pub use rustycog::http::AppState;

/// Standalone service prefix, mounted exactly once by the prefixed router.
pub const SERVICE_PREFIX: &str = "/oauth-connect";

/// Create the unprefixed health and per-instance v2 routes from a complete list.
///
/// Entries remain a vector so duplicates cannot disappear before validation.
/// The future router reuses the contract receiver rather than implementing HMAC.
///
/// # Errors
/// Always refuses in T0; no instance registry or routes are published.
pub fn create_router(
    _state: AppState,
    _entries: Vec<(String, ConnectState)>,
) -> Result<Router, DomainError> {
    Err(pending())
}

/// Create standalone routes under the service prefix, including readiness.
///
/// # Errors
/// Always refuses in T0; no provisional readiness or partial catalogue is exposed.
pub fn create_prefixed_router(
    _state: AppState,
    _entries: Vec<(String, ConnectState)>,
    _probe: Arc<ReadinessProbe>,
) -> Result<Router, DomainError> {
    Err(pending())
}

fn pending() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect HTTP registry construction not implemented".to_owned(),
    }
}
