//! Envoy HTTP Check: 200 with recreated principal, 403 for rejection, or 503
//! when session liveness cannot be established.

use crate::config::ExtAuthzConfig;
use crate::jwks_cache::{JwksCache, KeyStatus};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use jsonwebtoken::decode_header;
use rustycog::http::{
    HttpSessionLivenessChecker, SessionLivenessChecker, SessionLivenessError, UserIdExtractor,
};
use rustycog::permission::PrincipalPolicy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Check service state: local JWKS cache + optional test kid registry.
#[derive(Clone)]
pub struct AuthzState {
    cache: Arc<JwksCache>,
    audience: String,
    registry: HashMap<String, KeyStatus>,
    platform_issuer: String,
    session_liveness: Option<Arc<dyn SessionLivenessChecker>>,
}

impl AuthzState {
    /// # Errors
    ///
    /// Returns an error if the audience, JWKS cache or session-liveness
    /// configuration is invalid, or if liveness is enabled without an exact
    /// platform issuer.
    pub fn new(config: ExtAuthzConfig) -> Result<Self, String> {
        if config.audience.trim().is_empty() {
            return Err("EXT_AUTHZ_AUDIENCE is required (fail-closed)".into());
        }
        let cache = JwksCache::new(
            config.jwks_url,
            config.poll_interval,
            config.negative_cache_ttl,
        )?;
        let session_liveness: Option<Arc<dyn SessionLivenessChecker>> =
            HttpSessionLivenessChecker::from_config(&config.session_liveness)
                .map_err(|error| error.to_string())?
                .map(|checker| Arc::new(checker) as Arc<dyn SessionLivenessChecker>);
        if session_liveness.is_some() && config.platform_issuer.trim().is_empty() {
            return Err("EXT_AUTHZ_PLATFORM_ISSUER is required with session liveness".into());
        }
        Ok(Self {
            cache,
            audience: config.audience,
            platform_issuer: config.platform_issuer,
            session_liveness,
            registry: HashMap::new(),
        })
    }

    #[must_use]
    pub fn with_kid_status(mut self, kid: impl Into<String>, status: KeyStatus) -> Self {
        self.registry.insert(kid.into(), status);
        self
    }

    /// Attach an authoritative liveness reader for mesh Check.
    #[must_use]
    pub fn with_session_liveness_checker(
        mut self,
        checker: Arc<dyn SessionLivenessChecker>,
    ) -> Self {
        self.session_liveness = Some(checker);
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

/// HTTP Check router. `POST /` and `POST /check`, plus any other path/method
/// Envoy forwards (client path, including a doubled slash from `path_prefix: /`).
pub fn check_router(state: Arc<AuthzState>) -> Router {
    Router::new()
        .route("/", post(check))
        .route("/check", post(check))
        // Envoy http_service forwards the client path (and `path_prefix: /`
        // yields `//…`). Check is not a dedicated URL.
        .fallback(check)
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
        return deny("missing bearer token");
    };

    let header = match decode_header(&token) {
        Ok(h) => h,
        Err(err) => return deny(&format!("undecodable JWT header: {err}")),
    };
    let Some(kid) = header.kid.clone() else {
        return deny("JWT header without kid");
    };

    if state
        .status_override(&kid)
        .is_some_and(KeyStatus::is_denied)
    {
        return deny(&format!("kid {kid} denied by registry"));
    }

    let doc = match state.cache.document_for_kid(&kid).await {
        Ok(d) => d,
        Err(err) => return deny(&err),
    };

    if state
        .cache
        .status_for_kid(&kid)
        .is_some_and(KeyStatus::is_denied)
    {
        return deny(&format!("kid {kid} is pending or revoked in JWKS"));
    }

    let mut extractor = match UserIdExtractor::from_inline_jwks(&doc, Some(state.audience.as_str()))
    {
        Ok(e) => e,
        Err(err) => return deny(&format!("JWKS rejected: {err}")),
    };
    if !state.platform_issuer.is_empty() {
        let policy = match PrincipalPolicy::new(state.platform_issuer.clone(), false) {
            Ok(policy) => policy,
            Err(err) => return deny(&format!("platform policy rejected: {err}")),
        };
        extractor = match extractor.with_principal_policy(policy) {
            Ok(extractor) => extractor,
            Err(err) => return deny(&format!("platform policy rejected: {err}")),
        };
    }
    if let Some(checker) = state.session_liveness.clone() {
        extractor = extractor.with_session_liveness_checker(checker);
    }

    match extractor.extract_verified_principal(&token).await {
        Ok(verified) if state.cache.still_authorizes(&kid, &doc) => {
            match extractor
                .check_session_liveness(&verified.principal, verified.auth_epoch)
                .await
            {
                Ok(()) => {
                    // A platform check can await the authority. Recheck token
                    // validity using the same inline keys, then check that the
                    // cache still authorizes those keys before emitting allow.
                    if state.session_liveness.is_some()
                        && verified.principal.iss == state.platform_issuer
                        && extractor.extract_verified_principal(&token).await.is_err()
                    {
                        return deny("JWT is no longer valid after session liveness");
                    }
                    if !state.cache.still_authorizes(&kid, &doc) {
                        return deny("JWKS authorization snapshot expired or replaced");
                    }
                    allow(&verified.principal.iss, &verified.principal.sub.to_string())
                }
                Err(SessionLivenessError::Revoked) => deny("session is revoked"),
                Err(SessionLivenessError::Unavailable { .. }) => {
                    debug!("session liveness unavailable");
                    (StatusCode::SERVICE_UNAVAILABLE, HeaderMap::new())
                }
            }
        }
        Ok(_) => deny("JWKS authorization snapshot expired or replaced"),
        Err(err) => deny(&format!("JWT rejected: {err}")),
    }
}

fn deny(reason: &str) -> (StatusCode, HeaderMap) {
    debug!(reason, "check denied");
    (StatusCode::FORBIDDEN, HeaderMap::new())
}

fn allow(iss: &str, sub: &str) -> (StatusCode, HeaderMap) {
    let Ok(iss_value) = HeaderValue::from_str(iss) else {
        return deny("iss is not a valid header value");
    };
    let Ok(sub_value) = HeaderValue::from_str(sub) else {
        return deny("sub is not a valid header value");
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
    use axum::Json;
    use rustycog::http::SessionLivenessQuery;
    use rustycog::testing::http::jwt::{
        create_rs256_jwt_token, create_rs256_jwt_token_with_options,
        test_rs256_jwks_json as legacy_test_jwks, Rs256TokenOptions, TEST_JWT_AUDIENCE,
        TEST_PLATFORM_ISSUER, TEST_RS256_KID,
    };
    use serde::Deserialize;
    use serial_test::serial;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;
    use uuid::Uuid;

    #[derive(Debug, Clone, Copy)]
    enum LivenessBehavior {
        Live,
        LiveAfter(Duration),
        Revoked,
        Unavailable,
    }

    #[derive(Clone)]
    struct LivenessHttpFixture {
        behavior: LivenessBehavior,
        calls: Arc<AtomicUsize>,
        queries: Arc<Mutex<Vec<SessionLivenessQuery>>>,
    }

    #[derive(Deserialize)]
    struct LivenessHttpRequest {
        issuer: String,
        subject: Uuid,
        auth_epoch: Option<String>,
    }

    async fn liveness_handler(
        State(fixture): State<LivenessHttpFixture>,
        Json(request): Json<LivenessHttpRequest>,
    ) -> (StatusCode, Json<Value>) {
        fixture.calls.fetch_add(1, Ordering::SeqCst);
        fixture
            .queries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(SessionLivenessQuery {
                issuer: request.issuer,
                subject: request.subject,
                auth_epoch: request
                    .auth_epoch
                    .map(|value| value.parse().expect("canonical fixture epoch")),
            });
        if let LivenessBehavior::LiveAfter(delay) = fixture.behavior {
            tokio::time::sleep(delay).await;
        }
        match fixture.behavior {
            LivenessBehavior::Live | LivenessBehavior::LiveAfter(_) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "decision": "live",
                    "current_epoch": "0",
                })),
            ),
            LivenessBehavior::Revoked => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "decision": "revoked",
                    "current_epoch": "1",
                })),
            ),
            LivenessBehavior::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, Json(Value::Null)),
        }
    }

    fn liveness_server(
        behavior: LivenessBehavior,
    ) -> (
        axum_test::TestServer,
        String,
        Arc<AtomicUsize>,
        Arc<Mutex<Vec<SessionLivenessQuery>>>,
    ) {
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/liveness", post(liveness_handler))
            .with_state(LivenessHttpFixture {
                behavior,
                calls: calls.clone(),
                queries: queries.clone(),
            });
        let server = axum_test::TestServer::builder()
            .http_transport()
            .build(app)
            .unwrap();
        let endpoint = server.server_url("/liveness").unwrap().to_string();
        (server, endpoint, calls, queries)
    }

    fn test_rs256_jwks_json() -> String {
        let mut doc: Value = serde_json::from_str(&legacy_test_jwks()).expect("test JWKS");
        doc["keys"][0]["status"] = serde_json::json!("active");
        doc["keys"][0]["trust_scope"] = serde_json::json!("platform");
        doc["keys"][0]["organization_id"] = Value::Null;
        doc.to_string()
    }

    #[tokio::test]
    async fn pure_verifier_rejects_expired_bad_signature_future_dates_and_wrong_owner() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let valid = serde_json::json!({"sub":Uuid::new_v4().to_string(), "iss":TEST_PLATFORM_ISSUER,
            "aud":TEST_JWT_AUDIENCE, "exp":now+3600, "iat":now, "jti":"fixture-id"});
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(
            rustycog::testing::http::jwt::test_rs256_private_pem().as_bytes(),
        )
        .unwrap();
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.typ = Some("aiforall-access+jwt".into());
        header.kid = Some(TEST_RS256_KID.into());
        let sign = |claims: &Value| jsonwebtoken::encode(&header, claims, &key).unwrap();
        let extractor =
            UserIdExtractor::from_inline_jwks(test_rs256_jwks_json(), Some(TEST_JWT_AUDIENCE))
                .unwrap();
        assert!(extractor.extract_principal(&sign(&valid)).await.is_ok());
        for field in ["exp", "nbf", "iat"] {
            let mut claims = valid.clone();
            claims[field] = serde_json::json!(if field == "exp" {
                now - 3600
            } else {
                now + 3600
            });
            assert!(extractor.extract_principal(&sign(&claims)).await.is_err());
        }
        let mut parts: Vec<String> = sign(&valid).split('.').map(str::to_string).collect();
        let first = if parts[2].starts_with('A') { "B" } else { "A" };
        parts[2].replace_range(0..1, first);
        assert!(extractor.extract_principal(&parts.join(".")).await.is_err());
        let mut doc: Value = serde_json::from_str(&test_rs256_jwks_json()).unwrap();
        doc["keys"][0]["trust_scope"] = serde_json::json!("organization");
        doc["keys"][0]["organization_id"] = serde_json::json!(Uuid::new_v4().to_string());
        let extractor =
            UserIdExtractor::from_inline_jwks(doc.to_string(), Some(TEST_JWT_AUDIENCE)).unwrap();
        let mut claims = valid;
        claims["org"] = serde_json::json!(Uuid::new_v4().to_string());
        assert!(extractor.extract_principal(&sign(&claims)).await.is_err());
    }

    async fn server_with_jwks(jwks: &str) -> (axum_test::TestServer, JwksFixtures) {
        let fixture = JwksFixtures::service().await;
        fixture.mock_jwks_ok(jwks).await;
        let config = ExtAuthzConfig {
            jwks_url: format!("{}/.well-known/jwks.json", fixture.base_url()),
            audience: TEST_JWT_AUDIENCE.to_string(),
            platform_issuer: TEST_PLATFORM_ISSUER.to_string(),
            poll_interval: Duration::from_secs(3600),
            negative_cache_ttl: Duration::from_secs(30),
            ..ExtAuthzConfig::default()
        };
        let state = Arc::new(AuthzState::new(config).expect("state"));
        let server = axum_test::TestServer::new(check_router(state)).expect("server");
        (server, fixture)
    }

    async fn state_with_jwks(
        jwks: &str,
        poll_interval: Duration,
    ) -> (Arc<AuthzState>, JwksFixtures) {
        let fixture = JwksFixtures::isolated_service().await;
        fixture.mock_jwks_ok(jwks).await;
        let config = ExtAuthzConfig {
            jwks_url: format!("{}/.well-known/jwks.json", fixture.base_url()),
            audience: TEST_JWT_AUDIENCE.to_string(),
            platform_issuer: TEST_PLATFORM_ISSUER.to_string(),
            poll_interval,
            negative_cache_ttl: Duration::from_secs(30),
            ..ExtAuthzConfig::default()
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
    async fn denies_envoy_client_path_without_bearer() {
        let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
        for path in ["/deny-spoof", "//deny-spoof"] {
            let response = server
                .post(path)
                .add_header("x-principal-iss", "https://spoofed.example")
                .add_header("x-principal-sub", "00000000-0000-0000-0000-000000000000")
                .add_header("x-principal-org", "attacker-org")
                .await;
            response.assert_status(StatusCode::FORBIDDEN);
            assert!(response.maybe_header("x-principal-iss").is_none());
        }
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
            .fixture
            .server()
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.url.path().contains("jwks"))
            .count();
        assert_eq!(gets, 1, "second Check must not refetch JWKS");
    }

    #[tokio::test]
    #[serial]
    async fn rejects_wrong_audience() {
        #[derive(serde::Serialize)]
        struct Claims {
            sub: String,
            iss: String,
            aud: String,
            exp: usize,
            iat: usize,
            jti: String,
        }
        let user = Uuid::new_v4();
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
            rustycog::testing::http::jwt::test_rs256_private_pem().as_bytes(),
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
        let (state, jwks) = state_with_jwks(&test_rs256_jwks_json(), Duration::ZERO).await;
        let server = axum_test::TestServer::new(check_router(state.clone())).expect("server");

        server
            .post("/")
            .add_header("authorization", format!("Bearer {token}"))
            .await
            .assert_status(StatusCode::OK);

        let mut revoked = serde_json::from_str::<Value>(&test_rs256_jwks_json()).expect("jwks");
        revoked["keys"][0]["status"] = Value::String("revoked".into());
        let revoked_json = serde_json::to_string(&revoked).expect("json");
        jwks.fixture.reset().await;
        jwks.mock_jwks_ok(&revoked_json).await;

        // The bounded retry throttle does not reset snapshot freshness.
        tokio::time::sleep(Duration::from_millis(300)).await;
        state.poll_jwks_if_due().await;

        let gets_after_poll = jwks
            .fixture
            .server()
            .received_requests()
            .await
            .unwrap_or_default()
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
            .fixture
            .server()
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.url.path().contains("jwks"))
            .count();
        assert_eq!(
            gets_after_check, gets_after_poll,
            "Check after poll must not GET JWKS (known kid)"
        );
    }

    #[tokio::test]
    #[serial]
    async fn check_enforces_authoritative_liveness_before_recreating_principal() {
        for (behavior, expected) in [
            (LivenessBehavior::Live, StatusCode::OK),
            (LivenessBehavior::Revoked, StatusCode::FORBIDDEN),
            (
                LivenessBehavior::Unavailable,
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            let (_liveness, endpoint, calls, queries) = liveness_server(behavior);
            let checker = HttpSessionLivenessChecker::new(
                &endpoint,
                "x-workload-token",
                "fixture-secret",
                Duration::from_secs(2),
            )
            .unwrap();
            let (state, _jwks) =
                state_with_jwks(&test_rs256_jwks_json(), Duration::from_secs(3600)).await;
            state
                .cache
                .document_for_kid(TEST_RS256_KID)
                .await
                .expect("prime the fixture JWKS before Check");
            let state = state
                .as_ref()
                .clone()
                .with_session_liveness_checker(Arc::new(checker));
            assert!(state.session_liveness.is_some(), "checker must be attached");
            assert_eq!(state.platform_issuer, TEST_PLATFORM_ISSUER);
            let server = axum_test::TestServer::new(check_router(Arc::new(state))).unwrap();
            let user = Uuid::new_v4();
            let token = create_rs256_jwt_token(user);
            let response = server
                .post("/")
                .add_header("authorization", format!("Bearer {token}"))
                .await;
            if expected == StatusCode::OK {
                assert_eq!(
                    response.header("x-principal-iss").to_str().unwrap(),
                    TEST_PLATFORM_ISSUER
                );
                assert_eq!(
                    response.header("x-principal-sub").to_str().unwrap(),
                    user.to_string()
                );
            } else {
                assert!(response.maybe_header("x-principal-iss").is_none());
                assert!(response.maybe_header("x-principal-sub").is_none());
            }
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "one authoritative read for {behavior:?}"
            );
            let queries = queries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert_eq!(
                queries.as_slice(),
                &[SessionLivenessQuery {
                    issuer: TEST_PLATFORM_ISSUER.into(),
                    subject: user,
                    auth_epoch: None,
                }]
            );
            drop(queries);
            response.assert_status(expected);
        }
    }

    #[tokio::test]
    #[serial]
    async fn check_rejects_key_revoked_while_authority_is_pending() {
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let handler = {
            let entered = entered.clone();
            let release = release.clone();
            move || {
                let entered = entered.clone();
                let release = release.clone();
                async move {
                    entered.notify_one();
                    release.notified().await;
                    Json(serde_json::json!({"decision": "live", "current_epoch": "0"}))
                }
            }
        };
        let authority = axum_test::TestServer::builder()
            .http_transport()
            .build(Router::new().route("/liveness", post(handler)))
            .expect("local authority");
        let endpoint = authority.server_url("/liveness").expect("authority URL");
        let checker = HttpSessionLivenessChecker::new(
            endpoint.as_str(),
            "x-workload-token",
            "fixture-secret",
            Duration::from_secs(6),
        )
        .expect("authority client");

        let jwks = test_rs256_jwks_json();
        let (state, fixture) = state_with_jwks(&jwks, Duration::ZERO).await;
        state
            .cache
            .document_for_kid(TEST_RS256_KID)
            .await
            .expect("prime active key");
        let state = Arc::new(
            state
                .as_ref()
                .clone()
                .with_session_liveness_checker(Arc::new(checker)),
        );
        let server = axum_test::TestServer::new(check_router(state.clone())).expect("Check server");
        let token = create_rs256_jwt_token(Uuid::new_v4());
        let request = async {
            server
                .post("/check")
                .add_header("authorization", format!("Bearer {token}"))
                .await
        };
        let revoke = async {
            tokio::time::timeout(Duration::from_secs(2), entered.notified())
                .await
                .expect("verified Check reaches authority");
            let mut revoked: Value = serde_json::from_str(&jwks).expect("fixture JWKS");
            revoked["keys"][0]["status"] = Value::String("revoked".into());
            fixture.fixture.reset().await;
            fixture.mock_jwks_ok(&revoked.to_string()).await;
            tokio::time::timeout(Duration::from_secs(2), async {
                let mut ticks = tokio::time::interval(Duration::from_millis(50));
                loop {
                    ticks.tick().await;
                    state.poll_jwks_if_due().await;
                    if state.cache.status_for_kid(TEST_RS256_KID) == Some(KeyStatus::Revoked) {
                        break;
                    }
                }
            })
            .await
            .expect("revoked key applied before authoritative result");
            release.notify_one();
        };
        let (response, ()) = tokio::join!(request, revoke);
        response.assert_status(StatusCode::FORBIDDEN);
        assert!(response.maybe_header("x-principal-iss").is_none());
        assert!(response.maybe_header("x-principal-sub").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn check_refuses_an_authority_allow_after_the_delivery_budget() {
        let (_authority, endpoint, calls, _) =
            liveness_server(LivenessBehavior::LiveAfter(Duration::from_millis(1_250)));
        let checker = HttpSessionLivenessChecker::new(
            &endpoint,
            "x-workload-token",
            "fixture-secret",
            Duration::from_secs(4),
        )
        .expect("authority client");
        let (state, _jwks) =
            state_with_jwks(&test_rs256_jwks_json(), Duration::from_secs(3600)).await;
        state
            .cache
            .document_for_kid(TEST_RS256_KID)
            .await
            .expect("prime active key");
        let state = state
            .as_ref()
            .clone()
            .with_session_liveness_checker(Arc::new(checker));
        let server =
            axum_test::TestServer::new(check_router(Arc::new(state))).expect("Check server");
        let token = create_rs256_jwt_token(Uuid::new_v4());
        let response = server
            .post("/check")
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(calls.load(Ordering::SeqCst), 1, "authority was queried");
        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.maybe_header("x-principal-iss").is_none());
        assert!(response.maybe_header("x-principal-sub").is_none());
    }

    #[test]
    #[serial]
    fn from_env_requires_audience_and_configures_jwks_polling() {
        std::env::remove_var("EXT_AUTHZ_AUDIENCE");
        std::env::remove_var("EXT_AUTHZ_PLATFORM_ISSUER");
        std::env::remove_var("EXT_AUTHZ_LIVENESS_URL");
        std::env::remove_var("EXT_AUTHZ_LIVENESS_TOKEN_ENV");
        std::env::remove_var("EXT_AUTHZ_LIVENESS_TOKEN_HEADER");
        std::env::remove_var("EXT_AUTHZ_LIVENESS_TIMEOUT_MS");
        std::env::set_var("EXT_AUTHZ_JWKS_URL", "http://example/.well-known/jwks.json");
        let err = ExtAuthzConfig::from_env().expect_err("empty audience must fail-closed");
        assert!(
            err.contains("EXT_AUTHZ_AUDIENCE") || err.contains("fail-closed"),
            "{err}"
        );
        std::env::set_var("EXT_AUTHZ_AUDIENCE", "aiforall");
        std::env::remove_var("EXT_AUTHZ_JWKS_POLL_SECS");
        let config = ExtAuthzConfig::from_env().expect("default configuration");
        assert_eq!(config.poll_interval, Duration::from_secs(60));
        std::env::set_var("EXT_AUTHZ_JWKS_POLL_SECS", "2");
        let config = ExtAuthzConfig::from_env().expect("local configuration");
        assert_eq!(config.poll_interval, Duration::from_secs(2));
        std::env::remove_var("EXT_AUTHZ_JWKS_POLL_SECS");
        std::env::remove_var("EXT_AUTHZ_AUDIENCE");
        std::env::remove_var("EXT_AUTHZ_JWKS_URL");
    }
}
