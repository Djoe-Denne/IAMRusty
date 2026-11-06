//! G-6: contract-boundary v2 tests, not OAuthConnect service integration tests.
//! Oracles: accepted contract §§3.1–3.2 and QA T6. The outbound trait alone is
//! faked; the nested Axum router, HMAC middleware and JSON boundary are real.
//! Run the two positive routing tests before interpreting any negative result.

#![cfg(feature = "server")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router};
use axum_test::{TestRequest, TestServer};
use idp_connect_contract::hmac::{
    sign, unix_timestamp_secs, HmacKey, SIGNATURE_HEADER, TIMESTAMP_HEADER,
};
use idp_connect_contract::server::{router_v2, ConnectState};
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile, AUTHORIZE_AUTHENTICATION_PATH, COMPLETE_AUTHORIZATION_PATH,
};

const ALPHA: &str = "/oauth-connect/alpha";
const BETA: &str = "/oauth-connect/beta";
const REDIRECT: &str = "https://iam.example/iam/api/auth/alpha/callback";
const NONCE: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
// RFC 7636 Appendix B: known coherent S256 challenge/verifier, not two arbitrary strings.
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const AUTHORIZE_BODY: &str = r#"{"redirect_uri":"https://iam.example/iam/api/auth/alpha/callback","state":"browser-state-sentinel","code_challenge":"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM","code_challenge_method":"S256","nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#;
const COMPLETE_BODY: &str = r#"{"code":"authorization-code-sentinel","redirect_uri":"https://iam.example/iam/api/auth/alpha/callback","code_verifier":"dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk","expected_nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#;

#[derive(Default)]
struct LegacyClient {
    calls: AtomicUsize,
}

#[async_trait]
impl FederatedOAuthClient for LegacyClient {
    async fn authorize(
        &self,
        _redirect_uri: &str,
        _state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(authorize_response("legacy"))
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(authentication("legacy").tokens)
    }

    async fn user_profile(
        &self,
        _access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(authentication("legacy").profile)
    }
}

struct RecordingClient {
    instance: &'static str,
    legacy: LegacyClient,
    authorizations: Mutex<Vec<AuthorizeAuthenticationRequest>>,
    completions: Mutex<Vec<CompleteAuthorizationRequest>>,
}

impl RecordingClient {
    fn new(instance: &'static str) -> Self {
        Self {
            instance,
            legacy: LegacyClient::default(),
            authorizations: Mutex::default(),
            completions: Mutex::default(),
        }
    }

    fn counts(&self) -> (usize, usize, usize) {
        (
            self.authorizations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len(),
            self.completions
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len(),
            self.legacy.calls.load(Ordering::SeqCst),
        )
    }
}

#[async_trait]
impl FederatedOAuthClient for RecordingClient {
    async fn authorize(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.legacy.authorize(redirect_uri, state).await
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.legacy.exchange_code(code, redirect_uri).await
    }

    async fn user_profile(
        &self,
        access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        self.legacy.user_profile(access_token).await
    }

    async fn authorize_authentication(
        &self,
        request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.authorizations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request.clone());
        Ok(authorize_response(self.instance))
    }

    async fn complete_authorization(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        self.completions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request.clone());
        Ok(authentication(self.instance))
    }
}

fn authorize_request() -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: REDIRECT.into(),
        state: "browser-state-sentinel".into(),
        code_challenge: Some(CHALLENGE.into()),
        code_challenge_method: Some("S256".into()),
        nonce: NONCE.into(),
    }
}

fn complete_request() -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        code: "authorization-code-sentinel".into(),
        redirect_uri: REDIRECT.into(),
        code_verifier: Some(VERIFIER.into()),
        expected_nonce: NONCE.into(),
    }
}

fn authorize_response(instance: &str) -> AuthorizeResponse {
    AuthorizeResponse {
        authorization_url: format!("https://vendor.example/{instance}/authorize"),
        scope: "openid profile email".into(),
    }
}

fn authentication(instance: &str) -> ProviderAuthentication {
    ProviderAuthentication {
        tokens: ProviderTokens {
            access_token: format!("access-token-sentinel-{instance}"),
            refresh_token: Some(format!("refresh-token-sentinel-{instance}")),
            expires_in: Some(3600),
        },
        profile: ProviderUserProfile {
            id: format!("subject-{instance}"),
            username: "contract-user".into(),
            email: Some("contract-user@example.test".into()),
            avatar_url: Some("https://vendor.example/avatar".into()),
            email_verified: true,
        },
    }
}

struct Fixture {
    server: TestServer,
    alpha: Arc<RecordingClient>,
    beta: Arc<RecordingClient>,
    alpha_key: HmacKey,
    beta_key: HmacKey,
}

fn fixture() -> Fixture {
    let alpha = Arc::new(RecordingClient::new("alpha"));
    let beta = Arc::new(RecordingClient::new("beta"));
    let alpha_key = HmacKey::new(b"alpha-contract-test-key");
    let beta_key = HmacKey::new(b"beta-contract-test-key");
    let app = Router::new()
        .nest(
            ALPHA,
            router_v2(ConnectState {
                hmac_key: alpha_key.clone(),
                client: alpha.clone(),
            }),
        )
        .nest(
            BETA,
            router_v2(ConnectState {
                hmac_key: beta_key.clone(),
                client: beta.clone(),
            }),
        );
    Fixture {
        server: TestServer::new(app).expect("contract test router"),
        alpha,
        beta,
        alpha_key,
        beta_key,
    }
}

fn signed_at(
    server: &TestServer,
    key: &HmacKey,
    path: &str,
    signed_path: &str,
    body: &str,
    timestamp: i64,
) -> TestRequest {
    let signature =
        sign(key.as_bytes(), "POST", signed_path, timestamp, body).expect("contract test HMAC");
    server
        .post(path)
        .add_header(TIMESTAMP_HEADER, timestamp.to_string())
        .add_header(SIGNATURE_HEADER, signature)
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(body.to_owned()))
}

fn signed(f: &Fixture, suffix: &str, body: &str) -> TestRequest {
    let path = format!("{ALPHA}{suffix}");
    signed_at(
        &f.server,
        &f.alpha_key,
        &path,
        &path,
        body,
        unix_timestamp_secs().expect("clock"),
    )
}

fn no_calls(f: &Fixture) {
    assert_eq!(f.alpha.counts(), (0, 0, 0), "alpha must not be invoked");
    assert_eq!(f.beta.counts(), (0, 0, 0), "beta must not be invoked");
}

fn assert_authorize_forwarded(request: &AuthorizeAuthenticationRequest) {
    assert!(
        request.redirect_uri == REDIRECT,
        "redirect must be preserved"
    );
    assert!(
        request.state == "browser-state-sentinel",
        "state must be preserved"
    );
    assert!(
        request.nonce == NONCE,
        "independent nonce must be preserved"
    );
    assert!(
        request.code_challenge.as_deref() == Some(CHALLENGE),
        "challenge must be preserved"
    );
    assert!(
        request.code_challenge_method.as_deref() == Some("S256"),
        "method must be preserved"
    );
}

fn assert_complete_forwarded(request: &CompleteAuthorizationRequest) {
    assert!(
        request.redirect_uri == REDIRECT,
        "redirect must be preserved"
    );
    assert!(
        request.code == "authorization-code-sentinel",
        "code must be preserved"
    );
    assert!(
        request.expected_nonce == NONCE,
        "expected nonce must be preserved"
    );
    assert!(
        request.code_verifier.as_deref() == Some(VERIFIER),
        "verifier must be preserved"
    );
}

fn assert_authentication(result: &ProviderAuthentication, instance: &str) {
    let expected = authentication(instance);
    assert!(
        result.tokens.access_token == expected.tokens.access_token,
        "access token must match instance"
    );
    assert!(
        result.tokens.refresh_token == expected.tokens.refresh_token,
        "refresh token must match instance"
    );
    assert_eq!(result.tokens.expires_in, Some(3600));
    assert!(
        result.profile.id == expected.profile.id,
        "subject must match instance"
    );
    assert!(
        result.profile.username == expected.profile.username,
        "username must be preserved"
    );
    assert!(
        result.profile.email == expected.profile.email,
        "email must be preserved"
    );
    assert!(
        result.profile.avatar_url == expected.profile.avatar_url,
        "avatar must be preserved"
    );
    assert!(
        result.profile.email_verified,
        "verification assertion must be preserved"
    );
}

// G-6/T0 witness: detects an unusable fake or mismatched public DTOs, independently of routing.
#[tokio::test]
async fn fake_v2_client_witness_preserves_context_and_returns_both_results() {
    let client = RecordingClient::new("alpha");
    let authorized = client
        .authorize_authentication(&authorize_request())
        .await
        .expect("fake authorize");
    assert!(
        authorized.authorization_url == "https://vendor.example/alpha/authorize",
        "fake authorize response"
    );
    let completed = client
        .complete_authorization(&complete_request())
        .await
        .expect("fake complete");
    assert_authentication(&completed, "alpha");
    assert_authorize_forwarded(
        &client
            .authorizations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)[0],
    );
    assert_complete_forwarded(
        &client
            .completions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)[0],
    );
    assert_eq!(client.counts(), (1, 1, 0));
}

// G-6: catches empty/wrong routes, stripped HMAC PATH, lost nonce/PKCE, and v1 delegation.
#[tokio::test]
async fn authorize_v2_full_path_signature_forwards_exact_authentication_context() {
    let f = fixture();
    let response = signed(&f, AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY).await;
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "v2 authorize must be routed"
    );
    let result: AuthorizeResponse = response.json();
    assert!(
        result.authorization_url == "https://vendor.example/alpha/authorize",
        "authorization URL must come from alpha"
    );
    assert!(
        result.scope == "openid profile email",
        "scope must be preserved"
    );
    assert_eq!(f.alpha.counts(), (1, 0, 0));
    assert_eq!(f.beta.counts(), (0, 0, 0));
    assert_authorize_forwarded(
        &f.alpha
            .authorizations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)[0],
    );
}

// G-6: catches split v1 token/profile fallback and accidental claims/ID-token exposure.
#[tokio::test]
async fn complete_v2_full_path_signature_returns_tokens_and_minimal_profile_together() {
    let f = fixture();
    let response = signed(&f, COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY).await;
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "v2 complete must be routed"
    );
    let result: ProviderAuthentication = response.json();
    assert_authentication(&result, "alpha");
    for forbidden in ["\"id_token\"", "\"raw_claims\"", "\"claims\"", "\"extra\""] {
        assert!(
            !response.text().contains(forbidden),
            "completion must not expose rich claims or ID token"
        );
    }
    assert_eq!(f.alpha.counts(), (0, 1, 0));
    assert_eq!(f.beta.counts(), (0, 0, 0));
    assert_complete_forwarded(
        &f.alpha
            .completions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)[0],
    );
}

// G-6: catches missing authentication, short PATH acceptance, stale timestamps and wrong keys.
#[tokio::test]
async fn both_v2_operations_reject_missing_wrong_stale_and_unprefixed_hmac_before_client() {
    let f = fixture();
    for (suffix, body) in [
        (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY),
        (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY),
    ] {
        let path = format!("{ALPHA}{suffix}");
        let missing = f
            .server
            .post(&path)
            .content_type("application/json")
            .bytes(axum::body::Bytes::from(body.to_owned()))
            .await;
        assert_eq!(
            missing.status_code(),
            StatusCode::UNAUTHORIZED,
            "missing HMAC must fail"
        );
        let bearer_only = f
            .server
            .post(&path)
            .add_header("Authorization", "Bearer user-jwt-is-not-s2s-authentication")
            .content_type("application/json")
            .bytes(axum::body::Bytes::from(body.to_owned()))
            .await;
        assert_eq!(
            bearer_only.status_code(),
            StatusCode::UNAUTHORIZED,
            "bearer cannot replace HMAC"
        );
        let now = unix_timestamp_secs().expect("clock");
        for (key, signed_path, timestamp) in [
            (&f.beta_key, path.as_str(), now),
            (&f.alpha_key, suffix, now),
            (&f.alpha_key, path.as_str(), now - 120),
        ] {
            let response = signed_at(&f.server, key, &path, signed_path, body, timestamp).await;
            assert_eq!(
                response.status_code(),
                StatusCode::UNAUTHORIZED,
                "invalid HMAC must fail"
            );
            no_calls(&f);
        }
    }
    no_calls(&f);
}

// G-6: catches a MAC calculated over a reconstructed JSON subset that omits nonce or PKCE.
#[tokio::test]
async fn signed_nonce_and_pkce_cannot_be_mutated_in_either_operation() {
    let f = fixture();
    for (suffix, body, protected) in [
        (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY, NONCE),
        (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY, CHALLENGE),
        (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY, NONCE),
        (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY, VERIFIER),
    ] {
        let path = format!("{ALPHA}{suffix}");
        let now = unix_timestamp_secs().expect("clock");
        let signature = sign(f.alpha_key.as_bytes(), "POST", &path, now, body).expect("test HMAC");
        let replacement = format!("B{}", &protected[1..]);
        let mutated = body.replace(protected, &replacement);
        let response = f
            .server
            .post(&path)
            .add_header(TIMESTAMP_HEADER, now.to_string())
            .add_header(SIGNATURE_HEADER, signature)
            .content_type("application/json")
            .bytes(axum::body::Bytes::from(mutated))
            .await;
        assert_eq!(
            response.status_code(),
            StatusCode::UNAUTHORIZED,
            "body mutation must fail authentication"
        );
        no_calls(&f);
    }
}

// G-6: both keys work only on their own nested router; no trying all known keys.
#[tokio::test]
async fn instance_keys_and_clients_are_isolated_for_authorize_and_complete() {
    let f = fixture();
    for (prefix, key, instance) in [(ALPHA, &f.alpha_key, "alpha"), (BETA, &f.beta_key, "beta")] {
        for (suffix, body) in [
            (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY),
            (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY),
        ] {
            let path = format!("{prefix}{suffix}");
            let response = signed_at(
                &f.server,
                key,
                &path,
                &path,
                body,
                unix_timestamp_secs().expect("clock"),
            )
            .await;
            assert_eq!(response.status_code(), StatusCode::OK, "own key must work");
            if suffix == COMPLETE_AUTHORIZATION_PATH {
                assert_authentication(&response.json::<ProviderAuthentication>(), instance);
            } else {
                let result: AuthorizeResponse = response.json();
                assert!(
                    result.authorization_url == authorize_response(instance).authorization_url,
                    "own client must answer"
                );
            }
        }
    }
    assert_eq!(f.alpha.counts(), (1, 1, 0));
    assert_eq!(f.beta.counts(), (1, 1, 0));
    for (prefix, foreign_key) in [(ALPHA, &f.beta_key), (BETA, &f.alpha_key)] {
        for (suffix, body) in [
            (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY),
            (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY),
        ] {
            let path = format!("{prefix}{suffix}");
            let response = signed_at(
                &f.server,
                foreign_key,
                &path,
                &path,
                body,
                unix_timestamp_secs().expect("clock"),
            )
            .await;
            assert_eq!(
                response.status_code(),
                StatusCode::UNAUTHORIZED,
                "foreign instance key must fail"
            );
            assert_eq!(f.alpha.counts(), (1, 1, 0));
            assert_eq!(f.beta.counts(), (1, 1, 0));
        }
    }
}

// G-6: required JSON fields use Axum's existing 422 deserialization semantics.
#[tokio::test]
async fn missing_nonce_fields_are_unprocessable_before_client() {
    let f = fixture();
    for (suffix, body, field) in [
        (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY, "nonce"),
        (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY, "expected_nonce"),
    ] {
        let missing = body.replace(&format!(",\"{field}\":\"{NONCE}\""), "");
        let response = signed(&f, suffix, &missing).await;
        assert_eq!(
            response.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "required nonce must not default"
        );
        no_calls(&f);
    }
}

// G-6: catches checking only presence, accepting padding/non-base64url, or wrong byte length.
#[tokio::test]
async fn malformed_nonce_fields_are_bad_requests_before_client() {
    let f = fixture();
    for nonce in [
        "".to_owned(),
        "A".repeat(42),
        "A".repeat(44),
        format!("{NONCE}="),
        format!("+{}", "A".repeat(42)),
        format!("{}B", "A".repeat(42)),
    ] {
        for (suffix, body) in [
            (AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_BODY),
            (COMPLETE_AUTHORIZATION_PATH, COMPLETE_BODY),
        ] {
            let malformed = body.replace(NONCE, &nonce);
            let response = signed(&f, suffix, &malformed).await;
            assert_eq!(
                response.status_code(),
                StatusCode::BAD_REQUEST,
                "nonce format must be validated"
            );
            no_calls(&f);
        }
    }
}

// G-6: transposes the existing 400 PKCE wire contract, not a new error envelope.
#[tokio::test]
async fn incomplete_non_s256_and_malformed_pkce_are_rejected_before_client() {
    let f = fixture();
    let malformed_authorizations = [
        AUTHORIZE_BODY.replace(",\"code_challenge_method\":\"S256\"", ""),
        AUTHORIZE_BODY.replace(&format!(",\"code_challenge\":\"{CHALLENGE}\""), ""),
        AUTHORIZE_BODY.replace("S256", "plain"),
        AUTHORIZE_BODY.replace(CHALLENGE, "short"),
        AUTHORIZE_BODY.replace(CHALLENGE, &format!("+{}", "A".repeat(42))),
    ];
    for body in malformed_authorizations {
        let response = signed(&f, AUTHORIZE_AUTHENTICATION_PATH, &body).await;
        assert_eq!(
            response.status_code(),
            StatusCode::BAD_REQUEST,
            "invalid challenge pair must fail"
        );
        no_calls(&f);
    }
    for verifier in [
        "a".repeat(42),
        "a".repeat(129),
        format!("{}+", "a".repeat(42)),
    ] {
        let body = COMPLETE_BODY.replace(VERIFIER, &verifier);
        let response = signed(&f, COMPLETE_AUTHORIZATION_PATH, &body).await;
        assert_eq!(
            response.status_code(),
            StatusCode::BAD_REQUEST,
            "invalid verifier must fail"
        );
        no_calls(&f);
    }
}

// G-6: prevents accidental v1 aliases, an exposed profile API or case-insensitive fallback.
#[tokio::test]
async fn unknown_instances_noncanonical_paths_and_legacy_aliases_remain_not_found() {
    let f = fixture();
    for path in [
        "/oauth-connect/missing/v2/complete",
        "/oauth-connect/Alpha/v2/authorize",
        "/oauth-connect/alpha/v2/profile",
        "/oauth-connect/alpha/v1/authorize",
        "/oauth-connect/alpha/v1/token",
        "/oauth-connect/alpha/v1/profile",
        "/oauth-connect/alpha/v2/token",
    ] {
        let response = signed_at(
            &f.server,
            &f.alpha_key,
            path,
            path,
            COMPLETE_BODY,
            unix_timestamp_secs().expect("clock"),
        )
        .await;
        assert_eq!(
            response.status_code(),
            StatusCode::NOT_FOUND,
            "unexposed route must stay absent"
        );
        no_calls(&f);
    }
}

// G-6/T0 characterization: defaults must fail closed even when PKCE is omitted.
#[tokio::test]
async fn legacy_v2_defaults_refuse_without_any_v1_call() {
    let client = LegacyClient::default();
    let mut authorize = authorize_request();
    let mut complete = complete_request();
    for with_pkce in [true, false] {
        if !with_pkce {
            authorize.code_challenge = None;
            authorize.code_challenge_method = None;
            complete.code_verifier = None;
        }
        assert!(
            matches!(
                client.authorize_authentication(&authorize).await,
                Err(FederatedOAuthError::Authorize)
            ),
            "legacy v2 authorize must refuse"
        );
        assert!(
            matches!(
                client.complete_authorization(&complete).await,
                Err(FederatedOAuthError::ExchangeCode)
            ),
            "legacy v2 complete must refuse"
        );
        assert_eq!(
            client.calls.load(Ordering::SeqCst),
            0,
            "v2 defaults must not compose legacy calls"
        );
    }
}

// G-6/T0 characterization: catches derive(Debug) or nested token Debug leaking secrets.
#[test]
fn v2_sensitive_dto_debug_is_redacted() {
    let debug_values = [
        format!("{:?}", authorize_request()),
        format!("{:?}", complete_request()),
        format!("{:?}", authentication("alpha")),
        format!("{:?}", authentication("alpha").tokens),
    ];
    for debug in debug_values {
        for secret in [
            "browser-state-sentinel",
            "authorization-code-sentinel",
            NONCE,
            CHALLENGE,
            VERIFIER,
            "access-token-sentinel",
            "refresh-token-sentinel",
        ] {
            assert!(
                !debug.contains(secret),
                "sensitive DTO Debug must not disclose context or credentials"
            );
        }
    }
}

// G-6/T0 characterization: Serialize is exercised without adding serde_json dependency.
#[tokio::test]
async fn absent_optional_pkce_fields_are_omitted_from_v2_json() {
    let mut authorize = authorize_request();
    authorize.code_challenge = None;
    authorize.code_challenge_method = None;
    let mut complete = complete_request();
    complete.code_verifier = None;
    let authorize_body = to_bytes(Json(authorize).into_response().into_body(), 4096)
        .await
        .expect("serialize authorize");
    let complete_body = to_bytes(Json(complete).into_response().into_body(), 4096)
        .await
        .expect("serialize complete");
    let authorize_json = std::str::from_utf8(&authorize_body).expect("JSON UTF-8");
    let complete_json = std::str::from_utf8(&complete_body).expect("JSON UTF-8");
    assert!(
        !authorize_json.contains("\"code_challenge\""),
        "absent challenge must be omitted"
    );
    assert!(
        !authorize_json.contains("\"code_challenge_method\""),
        "absent method must be omitted"
    );
    assert!(
        !complete_json.contains("\"code_verifier\""),
        "absent verifier must be omitted"
    );
    assert!(
        authorize_json.contains("\"nonce\":"),
        "required nonce must remain serialized"
    );
    assert!(
        complete_json.contains("\"expected_nonce\":"),
        "required expected nonce must remain serialized"
    );
}
