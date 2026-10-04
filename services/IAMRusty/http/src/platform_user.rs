//! Human-account boundary: a bare UUID is not a platform identity.

use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}};
use rustycog::http::JwtPrincipal;
use uuid::Uuid;

/// Expected issuer, resolved once from the signing configuration at boot.
#[derive(Clone, Debug)]
pub struct PlatformIssuer(String);

impl PlatformIssuer {
    /// Reject an unset issuer rather than accepting legacy anonymous UUIDs.
    ///
    /// # Errors
    /// Returns an error for an empty or whitespace-padded issuer.
    pub fn new(issuer: String) -> Result<Self, &'static str> {
        if issuer.is_empty() || issuer.trim() != issuer {
            return Err("platform issuer must be explicitly configured");
        }
        Ok(Self(issuer))
    }
}

/// Extract only platform human accounts, never organization or mesh identities.
#[derive(Debug, Clone)]
pub struct PlatformUser {
    pub user_id: Uuid,
}

impl<S: Send + Sync> FromRequestParts<S> for PlatformUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let expected = parts.extensions.get::<PlatformIssuer>()
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let principal = parts.extensions.get::<JwtPrincipal>()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        if principal.iss != expected.0 || principal.org.is_some() {
            return Err(StatusCode::FORBIDDEN);
        }
        if crate::rate_limit::is_limited_path(parts.uri.path()) {
            let limiter = parts.extensions.get::<std::sync::Arc<crate::rate_limit::AuthRateLimiter>>()
                .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            if !limiter.take_authenticated_account(principal.sub) {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
        Ok(Self { user_id: principal.sub })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn extract(iss: &str, org: Option<&str>) -> Result<PlatformUser, StatusCode> {
        let (mut parts, _) = axum::http::Request::new(()).into_parts();
        parts.extensions.insert(PlatformIssuer::new("https://platform/iam".into()).unwrap());
        parts.extensions.insert(JwtPrincipal {
            iss: iss.into(), sub: Uuid::nil(), org: org.map(str::to_owned),
        });
        PlatformUser::from_request_parts(&mut parts, &()).await
    }

    #[tokio::test]
    async fn organization_issuer_cannot_impersonate_victim_uuid() {
        assert_eq!(extract("https://platform/iam/orgs/attacker", Some("attacker")).await.unwrap_err(), StatusCode::FORBIDDEN);
        assert_eq!(extract("https://platform/iam", Some("attacker")).await.unwrap_err(), StatusCode::FORBIDDEN);
        assert_eq!(extract("https://platform/iam", None).await.unwrap().user_id, Uuid::nil());
    }

    #[tokio::test]
    async fn injected_uuid_alone_is_not_identity_proof() {
        let (mut parts, _) = axum::http::Request::new(()).into_parts();
        parts.extensions.insert(PlatformIssuer::new("https://platform/iam".into()).unwrap());
        parts.extensions.insert(Uuid::nil());
        assert_eq!(PlatformUser::from_request_parts(&mut parts, &()).await.unwrap_err(), StatusCode::UNAUTHORIZED);
    }
}
