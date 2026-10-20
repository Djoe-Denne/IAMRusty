//! Service-origin proof comes only from the accepted TLS leaf, never headers.

use apparatus_contracts::runtime::{
    RuntimeControlCallerV1, RuntimeControlErrorV1, RuntimeEffectOutcomeV1, RuntimeErrorCodeV1,
    RuntimeServiceRoleV1,
};
use axum::{
    extract::{MatchedPath, Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
use rustycog::http::PeerClientCertificate;
use std::sync::Arc;

/// Setup-owned verifier over the transport-proven accepted leaf DER.
pub trait RuntimeServiceOriginAuthenticator: Send + Sync {
    /// Authenticate a service principal, with no header/body/loopback fallback.
    ///
    /// # Errors
    /// Returns unauthenticated for missing/bad proof, forbidden for another SAN.
    fn authenticate(
        &self,
        peer: Option<&PeerClientCertificate>,
    ) -> Result<RuntimeControlCallerV1, RuntimeControlErrorV1>;
}
/// Explicit composition dependency, not a per-handler verifier factory.
pub type RuntimeServiceOriginAuth = Arc<dyn RuntimeServiceOriginAuthenticator>;

/// Embedding/monolith has no equivalent transport attestation by default.
pub struct RejectRuntimeServiceOrigin;
impl RuntimeServiceOriginAuthenticator for RejectRuntimeServiceOrigin {
    fn authenticate(
        &self,
        _peer: Option<&PeerClientCertificate>,
    ) -> Result<RuntimeControlCallerV1, RuntimeControlErrorV1> {
        Err(failure(RuntimeErrorCodeV1::RuntimeUnauthenticated))
    }
}

pub(crate) async fn authenticate_runtime_origin(
    State(auth): State<RuntimeServiceOriginAuth>,
    mut request: Request,
    next: Next,
) -> Result<Response, super::handlers::runtime_control::RuntimeControlHttpError> {
    // Never trust a preexisting caller extension from another middleware/embedding.
    request.extensions_mut().remove::<RuntimeControlCallerV1>();
    let caller = auth.authenticate(request.extensions().get::<PeerClientCertificate>())?;
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_default();
    if !route_allowed(caller.role, request.method(), path) {
        return Err(failure(RuntimeErrorCodeV1::RuntimeServiceForbidden).into());
    }
    request.extensions_mut().insert(caller);
    Ok(next.run(request).await)
}

fn route_allowed(role: RuntimeServiceRoleV1, method: &Method, path: &str) -> bool {
    match (method.as_str(), path) {
        ("GET", path)
            if path.ends_with("/bindings") || path.ends_with("/bindings/{binding_id}") =>
        {
            true
        }
        ("POST", path) if path.ends_with("/writer-lease") => {
            role == RuntimeServiceRoleV1::Projector
        }
        ("PUT", path)
            if path.ends_with("/observation") || path.ends_with("/instances/{instance_id}") =>
        {
            role == RuntimeServiceRoleV1::Projector
        }
        ("POST", path)
            if path.ends_with("/instances/{instance_id}/revoke")
                || path.ends_with("/effects/begin")
                || path.ends_with("/effects/{effect_id}/complete") =>
        {
            role == RuntimeServiceRoleV1::Lazaret
        }
        _ => false,
    }
}

fn failure(code: RuntimeErrorCodeV1) -> RuntimeControlErrorV1 {
    code.error(
        RuntimeServiceRoleV1::Projector,
        RuntimeEffectOutcomeV1::NotStarted,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_route_role_matrix_and_default_rejection() {
        let matrix = [
            (Method::GET, "/bindings", None),
            (Method::GET, "/bindings/{binding_id}", None),
            (
                Method::POST,
                "/bindings/{binding_id}/writer-lease",
                Some(RuntimeServiceRoleV1::Projector),
            ),
            (
                Method::PUT,
                "/bindings/{binding_id}/observation",
                Some(RuntimeServiceRoleV1::Projector),
            ),
            (
                Method::PUT,
                "/bindings/{binding_id}/instances/{instance_id}",
                Some(RuntimeServiceRoleV1::Projector),
            ),
            (
                Method::POST,
                "/bindings/{binding_id}/instances/{instance_id}/revoke",
                Some(RuntimeServiceRoleV1::Lazaret),
            ),
            (
                Method::POST,
                "/bindings/{binding_id}/effects/begin",
                Some(RuntimeServiceRoleV1::Lazaret),
            ),
            (
                Method::POST,
                "/bindings/{binding_id}/effects/{effect_id}/complete",
                Some(RuntimeServiceRoleV1::Lazaret),
            ),
        ];
        for (method, suffix, required) in matrix {
            let path = format!("/manifesto/internal/apparatus/runtime{suffix}");
            for role in [
                RuntimeServiceRoleV1::Projector,
                RuntimeServiceRoleV1::Lazaret,
            ] {
                assert_eq!(
                    route_allowed(role, &method, &path),
                    required.is_none_or(|required| required == role)
                );
                assert!(!route_allowed(role, &Method::DELETE, &path));
            }
        }
        assert_eq!(
            RejectRuntimeServiceOrigin
                .authenticate(None)
                .err()
                .map(|error| error.code),
            Some(RuntimeErrorCodeV1::RuntimeUnauthenticated)
        );
    }
}
