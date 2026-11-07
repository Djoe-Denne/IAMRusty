//! Immutable instance routing at the existing v2 HMAC boundary.

use std::{collections::BTreeSet, sync::Arc};

use axum::Router;
use idp_connect_contract::server::router_v2;
pub use idp_connect_contract::server::ConnectState;
use readiness::attach_ready;
pub use readiness::ReadinessProbe;
pub use rustycog::core::error::DomainError;
pub use rustycog::http::AppState;
use rustycog::http::RouteBuilder;

/// Standalone service prefix, mounted exactly once by the prefixed router.
pub const SERVICE_PREFIX: &str = "/oauth-connect";

/// Create the unprefixed health and per-instance v2 routes from a complete list.
///
/// Entries remain a vector so duplicates cannot disappear before validation.
/// Static instance paths reuse the existing receiver and its full original-URI
/// HMAC check; no decoded slug lookup, wildcard or legacy fallback is installed.
///
/// # Errors
/// Refuses empty catalogues, duplicate slugs and noncanonical slugs without
/// exposing configuration data or publishing a partial set of routes.
pub fn create_router(
    state: AppState,
    entries: Vec<(String, ConnectState)>,
) -> Result<Router, DomainError> {
    validate_entries(&entries)?;
    let mut router = RouteBuilder::new(state).health_check().into_router();
    for (id, connect) in entries {
        // Validation permits only lowercase ASCII letters, never route syntax.
        // Axum matches these static paths against the raw URI path, so encoded,
        // uppercase, repeated-slash and dot-segment aliases do not gain a route.
        router = router.nest(&format!("/{id}"), router_v2(connect));
    }
    Ok(router)
}

/// Create standalone routes under the service prefix, including readiness.
///
/// # Errors
/// Returns the same sanitized catalogue errors as [`create_router`]. Readiness
/// uses only the supplied platform probe and performs no vendor operation.
pub fn create_prefixed_router(
    state: AppState,
    entries: Vec<(String, ConnectState)>,
    probe: Arc<ReadinessProbe>,
) -> Result<Router, DomainError> {
    let router = create_router(state, entries)?;
    Ok(Router::new().nest(SERVICE_PREFIX, attach_ready(router, probe)))
}

fn validate_entries(entries: &[(String, ConnectState)]) -> Result<(), DomainError> {
    let mut seen = BTreeSet::new();
    if entries.is_empty()
        || entries.iter().any(|(id, _)| {
            id.is_empty()
                || id.len() > 50
                || !id.bytes().all(|byte| byte.is_ascii_lowercase())
                || !seen.insert(id.as_str())
        })
    {
        return Err(DomainError::InvalidInput {
            message: "Invalid OAuthConnect HTTP registry".to_owned(),
        });
    }
    Ok(())
}
