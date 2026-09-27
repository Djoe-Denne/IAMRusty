//! Envoy HTTP Check: POST original headers → 200 + recreated principal, or 403.

use crate::config::ExtAuthzConfig;
use crate::jwks_cache::{JwksCache, KeyStatus};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use jsonwebtoken::decode_header;
use rustycog::http::UserIdExtractor;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Check service state: local JWKS cache + optional test kid registry.
pub struct AuthzState {
    cache: Arc<JwksCache>,
    audience: String,
    registry: HashMap<String, KeyStatus>,
}

impl AuthzState {
    /// # Errors
    ///
    /// Returns an error if the JWKS cache cannot be constructed.
    pub fn new(config: ExtAuthzConfig) -> Result<Self, String> {
        if config.audience.trim().is_empty() {
            return Err("EXT_AUTHZ_AUDIENCE is required (fail-closed)".into());
        }
        let cache = JwksCache::new(
            config.jwks_url,
            config.poll_interval,
            config.negative_cache_ttl,
        )?;
        Ok(Self {
            cache,
            audience: config.audience,
            registry: HashMap::new(),
        })
    }

    #[must_use]
    pub fn with_kid_status(mut self, kid: impl Into<String>, status: KeyStatus) -> Self {
        self.registry.insert(kid.into(), status);
        self
    }

    /// Background JWKS poll (bin loop). Not invoked on Check when `kid` is known.
    pub async fn poll_jwks_if_due(&self) {
        self.cache.poll_if_due().await;
    }

    fn status_override(&self, kid: &str) -> Option<KeyStatus> {
        self.registry.get(kid).copied()
    }
}

/// HTTP Check router (`POST /` and `POST /check`).
#[must_use]
pub fn check_router(state: Arc<AuthzState>) -> Router {
    Router::new()
        .route("/", post(check))
        .route("/check", post(check))
        .with_state(state)
}

async fn check(
    State(state): State<Arc<AuthzState>>,
    incoming: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let mut headers = incoming;
    merge_check_body_headers(&mut headers, &body);
    strip_all_x_principal(&mut headers);

    let Some(token) = bearer_token(&headers) else {
        return deny();
    };

    let header = match decode_header(&token) {
        Ok(h) => h,
        Err(_) => return deny(),
    };
    let Some(kid) = header.kid.clone() else {
        return deny();
    };

    if state
        .status_override(&kid)
        .is_some_and(KeyStatus::is_denied)
    {
        return deny();
    }

    let doc = match state.cache.document_for_kid(&kid).await {
        Ok(d) => d,
        Err(_) => return deny(),
    };

    if state
        .cache
        .status_for_kid(&kid)
        .is_some_and(KeyStatus::is_denied)
    {
        return deny();
    }

    let extractor = match UserIdExtractor::from_inline_jwks(&doc, Some(state.audience.as_str())) {
        Ok(e) => e,
        Err(_) => return deny(),
    };

    match extractor.extract_principal(&token).await {
        Ok(principal) => allow(&principal.iss, &principal.sub.to_string()),
        Err(_) => deny(),
    }
}

fn deny() -> (StatusCode, HeaderMap) {
    (StatusCode::FORBIDDEN, HeaderMap::new())
}

fn allow(iss: &str, sub: &str) -> (StatusCode, HeaderMap) {
    let Ok(iss_value) = HeaderValue::from_str(iss) else {
        return deny();
    };
    let Ok(sub_value) = HeaderValue::from_str(sub) else {
        return deny();
    };
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("x-principal-iss"), iss_value);
    headers.insert(HeaderName::from_static("x-principal-sub"), sub_value);
    (StatusCode::OK, headers)
}

fn strip_all_x_principal(headers: &mut HeaderMap) {
    let doomed: Vec<HeaderName> = headers
        .keys()
        .filter(|k| k.as_str().to_ascii_lowercase().starts_with("x-principal-"))
        .cloned()
        .collect();
    for name in doomed {
        headers.remove(&name);
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok())?;
    let token = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn merge_check_body_headers(headers: &mut HeaderMap, body: &Bytes) {
    if body.is_empty() {
        return;
    }
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return;
    };
    let Some(map) = value
        .pointer("/attributes/request/http/headers")
        .and_then(Value::as_object)
    else {
        return;
    };
    for (key, val) in map {
        if key.to_ascii_lowercase().starts_with("x-principal-") {
            continue;
        }
        if headers.keys().any(|k| k.as_str().eq_ignore_ascii_case(key)) {
            continue;
        }
        if let Some(s) = val.as_str() {
            if let (Ok(name), Ok(hv)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(s),
            ) {
                headers.insert(name, hv);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwks_fixtures::JwksFixtures;
    use rustycog::testing::http::jwt::{
        create_rs256_jwt_token, create_rs256_jwt_token_with_options, test_rs256_jwks_json,
        Rs256TokenOptions, TEST_JWT_AUDIENCE, TEST_PLATFORM_ISSUER, TEST_RS256_KID,
    };
    use serial_test::serial;
    use std::time::Duration;
    use uuid::Uuid;

    async fn server_with_jwks(jwks: &str) -> (axum_test::TestServer, JwksFixtures) {
        let fixture = JwksFixtures::service().await;
        fixture.mock_jwks_ok(jwks).await;
        let config = ExtAuthzConfig {
            jwks_url: format!("{}/.well-known/jwks.json", fixture.base_url()),
            audience: TEST_JWT_AUDIENCE.to_string(),
            poll_interval: Duration::from_secs(3600),
            negative_cache_ttl: Duration::from_secs(30),
        };
        let state = Arc::new(AuthzState::new(config).expect("state"));
        let server = axum_test::TestServer::new(check_router(state)).expect("server");
        (server, fixture)
    }

    async fn state_with_jwks(
        jwks: &str,
        poll_interval: Duration,
    ) -> (Arc<AuthzState>, JwksFixtures) {
        let fixture = JwksFixtures::service().await;
        fixture.mock_jwks_ok(jwks).await;
        let config = ExtAuthzConfig {
            jwks_url: format!("{}/.well-known/jwks.json", fixture.base_url()),
            audience: TEST_JWT_AUDIENCE.to_string(),
            poll_interval,
            negative_cache_ttl: Duration::from_secs(30),
        };
        let state = Arc::new(AuthzState::new(config).expect("state"));
        (state, fixture)
    }

    #[tokio::test]
    #[serial]
    async fn strips_client_x_principal_and_recreates_iss_sub() {
        let user = Uuid::new_v4();
        let token = create_rs256_jwt_token(user);
        let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;

        let response = server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .add_header("x-principal-iss", "https://spoofed.example")
            .add_header("x-principal-sub", "00000000-0000-0000-0000-000000000000")
            .add_header("x-principal-org", "attacker-org")
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.header("x-principal-iss").to_str().expect("iss"),
            TEST_PLATFORM_ISSUER
        );
        assert_eq!(
            response.header("x-principal-sub").to_str().expect("sub"),
            user.to_string()
        );
        assert!(response.maybe_header("x-principal-org").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn rejects_spoofed_x_principal_without_bearer() {
        let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
        let response = server
            .post("/")
            .add_header("x-principal-iss", TEST_PLATFORM_ISSUER)
            .add_header("x-principal-sub", Uuid::new_v4().to_string())
            .await;
        response.assert_status(StatusCode::FORBIDDEN);
        assert!(response.maybe_header("x-principal-iss").is_none());
        assert!(response.maybe_header("x-principal-sub").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn rejects_bad_iss_or_revoked_kid() {
        let user = Uuid::new_v4();
        let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
        let bad_iss = create_rs256_jwt_token_with_options(
            user,
            Rs256TokenOptions {
                iss: Some("https://evil.example/iam"),
                ..Rs256TokenOptions::default()
            },
        );
        let denied = server
            .post("/")
            .add_header("authorization", format!("Bearer {bad_iss}"))
            .await;
        denied.assert_status(StatusCode::FORBIDDEN);

        let mut revoked = serde_json::from_str::<Value>(&test_rs256_jwks_json()).expect("jwks");
        revoked["keys"][0]["status"] = Value::String("revoked".into());
        let revoked_json = serde_json::to_string(&revoked).expect("json");
        let (revoked_server, _f) = server_with_jwks(&revoked_json).await;
        let ok_token = create_rs256_jwt_token(user);
        let revoked_resp = revoked_server
            .post("/")
            .add_header("authorization", format!("Bearer {ok_token}"))
            .await;
        revoked_resp.assert_status(StatusCode::FORBIDDEN);

        let pending = {
            let mut v = serde_json::from_str::<Value>(&test_rs256_jwks_json()).expect("jwks");
            v["keys"][0]["status"] = Value::String("pending".into());
            serde_json::to_string(&v).expect("json")
        };
        let (pending_server, _f) = server_with_jwks(&pending).await;
        let pending_resp = pending_server
            .post("/")
            .add_header(
                "authorization",
                format!("Bearer {}", create_rs256_jwt_token(user)),
            )
            .await;
        pending_resp.assert_status(StatusCode::FORBIDDEN);
        let _ = TEST_RS256_KID;
    }

    #[tokio::test]
    #[serial]
    async fn jwks_second_request_does_not_refetch() {
        let user = Uuid::new_v4();
        let token = create_rs256_jwt_token(user);
        let (server, jwks) = server_with_jwks(&test_rs256_jwks_json()).await;

        server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .await
            .assert_status(StatusCode::OK);
        server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .await
            .assert_status(StatusCode::OK);

        let gets = jwks
            .received_requests()
            .await
            .into_iter()
            .filter(|r| r.url.path().contains("jwks"))
            .count();
        assert_eq!(gets, 1, "second Check must not refetch JWKS");
    }

    #[tokio::test]
    #[serial]
    async fn rejects_wrong_audience() {
        let user = Uuid::new_v4();
        #[derive(serde::Serialize)]
        struct Claims {
            sub: String,
            iss: String,
            aud: String,
            exp: usize,
            iat: usize,
            jti: String,
        }
        let claims = Claims {
            sub: user.to_string(),
            iss: TEST_PLATFORM_ISSUER.to_string(),
            aud: "https://wrong.audience.example".to_string(),
            exp: 4_102_444_800, // ~2100-01-01
            iat: 1_700_000_000,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.typ = Some("aiforall-access+jwt".into());
        header.kid = Some(TEST_RS256_KID.to_string());
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(
            rustycog::testing::http::jwt::TEST_RS256_PRIVATE_PEM.as_bytes(),
        )
        .expect("pem");
        let token = jsonwebtoken::encode(&header, &claims, &key).expect("encode");

        let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
        let response = server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        response.assert_status(StatusCode::FORBIDDEN);
        assert!(response.maybe_header("x-principal-iss").is_none());
        assert!(response.maybe_header("x-principal-sub").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn poll_revokes_kid_without_check_refetch() {
        let user = Uuid::new_v4();
        let token = create_rs256_jwt_token(user);
        let (state, jwks) =
            state_with_jwks(&test_rs256_jwks_json(), Duration::ZERO).await;
        let server = axum_test::TestServer::new(check_router(state.clone())).expect("server");

        server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .await
            .assert_status(StatusCode::OK);

        let mut revoked = serde_json::from_str::<Value>(&test_rs256_jwks_json()).expect("jwks");
        revoked["keys"][0]["status"] = Value::String("revoked".into());
        let revoked_json = serde_json::to_string(&revoked).expect("json");
        jwks.reset().await;
        jwks.mock_jwks_ok(&revoked_json).await;

        // refresh_coalesced skips re-fetch within 50ms of last_fetch
        tokio::time::sleep(Duration::from_millis(60)).await;
        state.poll_jwks_if_due().await;

        let gets_after_poll = jwks
            .received_requests()
            .await
            .into_iter()
            .filter(|r| r.url.path().contains("jwks"))
            .count();
        assert!(
            gets_after_poll >= 1,
            "poll must GET JWKS after reset; got {gets_after_poll}"
        );

        let denied = server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        denied.assert_status(StatusCode::FORBIDDEN);

        let gets_after_check = jwks
            .received_requests()
            .await
            .into_iter()
            .filter(|r| r.url.path().contains("jwks"))
            .count();
        assert_eq!(
            gets_after_check, gets_after_poll,
            "Check after poll must not GET JWKS (known kid)"
        );
    }

    #[test]
    fn from_env_requires_audience() {
        std::env::remove_var("EXT_AUTHZ_AUDIENCE");
        std::env::set_var("EXT_AUTHZ_JWKS_URL", "http://example/.well-known/jwks.json");
        let err = ExtAuthzConfig::from_env().expect_err("empty audience must fail-closed");
        assert!(
            err.contains("EXT_AUTHZ_AUDIENCE") || err.contains("fail-closed"),
            "{err}"
        );
        std::env::remove_var("EXT_AUTHZ_JWKS_URL");
    }
}
