//! G-4/G-9: contract oracles, independent of the not-yet-implemented v2 adapter.
//! Private, ephemeral WireMock listeners reuse the existing v1 unit-test seam;
//! no shared fixed-port fixture, Docker, vendor OIDC claims or second harness.

use super::HttpIdpConnector;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use iam_configuration::security::SecurityMode;
use iam_domain::port::service::FederatedOAuthClient;
use idp_connect_contract::dto::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    ProviderAuthentication,
};
use idp_connect_contract::hmac::{sign, SIGNATURE_HEADER, TIMESTAMP_HEADER};
use idp_connect_contract::FederatedOAuthError;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

const SECRET: &str = "iam-v2-wire-test-hmac-sentinel";
const CODE: &str = "IAM-V2-CODE-SENTINEL";
const ACCESS: &str = "IAM-V2-ACCESS-SENTINEL";
const REFRESH: &str = "IAM-V2-REFRESH-SENTINEL";
const REMOTE_ERROR: &str = "IAM-V2-REMOTE-ERROR-SENTINEL";
const REDIRECT: &str = "https://iam.example/oauth/callback";
const LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy)]
enum Operation {
    Authorize,
    Complete,
}

impl Operation {
    fn suffix(self) -> &'static str {
        match self {
            Self::Authorize => "/v2/authorize",
            Self::Complete => "/v2/complete",
        }
    }

    fn error(self) -> FederatedOAuthError {
        match self {
            Self::Authorize => FederatedOAuthError::Authorize,
            Self::Complete => FederatedOAuthError::ExchangeCode,
        }
    }

    fn response(self) -> Value {
        match self {
            Self::Authorize => json!({
                "authorization_url": "https://vendor.example/authorize?state=iam-v2-state",
                "scope": "openid profile email"
            }),
            Self::Complete => json!({
                "tokens": {"access_token": ACCESS, "refresh_token": REFRESH, "expires_in": 3600},
                "profile": {"id": "00042-opaque-sub", "username": "iam-v2-user",
                    "email": "iam-v2@example.test", "avatar_url": "https://vendor.example/avatar",
                    "email_verified": true}
            }),
        }
    }
}

fn nonce() -> String {
    // 43 base64url characters, canonical final sextet: decodes to exactly 32 bytes.
    let nonce = format!("{}A", "N".repeat(42));
    let decoded = URL_SAFE_NO_PAD
        .decode(&nonce)
        .expect("canonical nominal nonce fixture");
    assert_eq!(
        decoded.len(),
        32,
        "nominal nonce fixture is exactly 32 bytes"
    );
    assert!(
        URL_SAFE_NO_PAD.encode(decoded) == nonce,
        "canonical base64url round trip"
    );
    nonce
}

fn authorize_request() -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: REDIRECT.into(),
        state: "iam-v2-state".into(),
        code_challenge: Some("c".repeat(43)),
        code_challenge_method: Some("S256".into()),
        nonce: nonce(),
    }
}

fn complete_request() -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        code: CODE.into(),
        redirect_uri: REDIRECT.into(),
        code_verifier: Some("V".repeat(64)),
        expected_nonce: nonce(),
    }
}

fn client(server: &MockServer, slug: &str) -> HttpIdpConnector {
    HttpIdpConnector::with_security_mode(
        format!("{}/oauth-connect/{slug}/", server.uri()),
        SECRET,
        SecurityMode::IsolatedTest,
    )
    .expect("explicit isolated plaintext transport with valid key")
}

async fn mount(server: &MockServer, slug: &str, operation: Operation, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(format!("/oauth-connect/{slug}{}", operation.suffix())))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn receipts(server: &MockServer, slug: &str, operations: &[Operation]) -> Vec<Request> {
    let requests = server
        .received_requests()
        .await
        .expect("request recording enabled");
    assert_eq!(
        requests.len(),
        operations.len(),
        "exact outbound count; no retries, fallback or v1"
    );
    for (request, operation) in requests.iter().zip(operations) {
        assert_eq!(request.method.as_str(), "POST");
        assert_eq!(
            request.url.path(),
            format!("/oauth-connect/{slug}{}", operation.suffix())
        );
    }
    requests
}

fn sanitized(error: FederatedOAuthError, operation: Operation, server: &MockServer) {
    assert_eq!(
        error,
        operation.error(),
        "operation-specific generic category"
    );
    let diagnostic = format!("{error} {error:?}");
    for sensitive in [
        CODE,
        ACCESS,
        REFRESH,
        REMOTE_ERROR,
        SECRET,
        &nonce(),
        &"V".repeat(64),
        &server.uri(),
    ] {
        assert!(
            !diagnostic.contains(sensitive),
            "error must not expose request, response or destination"
        );
    }
}

async fn call(client: &HttpIdpConnector, operation: Operation) -> Result<(), FederatedOAuthError> {
    match operation {
        Operation::Authorize => {
            let result = client
                .authorize_authentication(&authorize_request())
                .await?;
            assert!(
                result.authorization_url == "https://vendor.example/authorize?state=iam-v2-state",
                "exact authorization URL"
            );
            assert_eq!(result.scope, "openid profile email");
        }
        Operation::Complete => {
            let result = client.complete_authorization(&complete_request()).await?;
            // Boolean comparisons deliberately avoid printing tokens in panic diagnostics.
            assert!(
                result.tokens.access_token == ACCESS,
                "access token preserved"
            );
            assert!(
                result.tokens.refresh_token.as_deref() == Some(REFRESH),
                "refresh token preserved"
            );
            assert_eq!(result.tokens.expires_in, Some(3600));
            assert_eq!(result.profile.id, "00042-opaque-sub");
            assert_eq!(result.profile.username, "iam-v2-user");
            assert_eq!(result.profile.email.as_deref(), Some("iam-v2@example.test"));
            assert_eq!(
                result.profile.avatar_url.as_deref(),
                Some("https://vendor.example/avatar")
            );
            assert!(result.profile.email_verified);
        }
    }
    Ok(())
}

fn signed_receipt(request: &Request) {
    let timestamp = request
        .headers
        .get(TIMESTAMP_HEADER)
        .expect("timestamp header")
        .to_str()
        .expect("timestamp ASCII")
        .parse::<i64>()
        .expect("timestamp number");
    let received = request
        .headers
        .get(SIGNATURE_HEADER)
        .expect("signature header")
        .to_str()
        .expect("signature ASCII");
    let body = std::str::from_utf8(&request.body).expect("UTF-8 JSON on wire");
    let expected = sign(
        SECRET.as_bytes(),
        "POST",
        request.url.path(),
        timestamp,
        body,
    )
    .expect("HMAC");
    assert!(
        received == expected,
        "HMAC covers full received PATH and raw received bytes"
    );
    for (path, body) in [
        ("/v2/authorize", body.to_owned()),
        (request.url.path(), format!("{body} ")),
    ] {
        let altered =
            sign(SECRET.as_bytes(), "POST", path, timestamp, &body).expect("HMAC mutation witness");
        assert!(
            received != altered,
            "receipt oracle detects PATH/body tampering"
        );
    }
}

// G-4/G-9; mutation: default refusal, wrong instance, unsigned field, token/profile fallback.
#[tokio::test]
async fn v2_two_slugs_sign_exact_raw_bodies_and_return_grouped_identity_without_v1() {
    for slug in ["research-east", "private-42"] {
        let server = MockServer::start().await;
        let client = client(&server, slug);
        for operation in [Operation::Authorize, Operation::Complete] {
            mount(
                &server,
                slug,
                operation,
                ResponseTemplate::new(200).set_body_json(operation.response()),
            )
            .await;
            call(&client, operation)
                .await
                .expect("valid v2 fixture must succeed");
        }
        let requests = receipts(&server, slug, &[Operation::Authorize, Operation::Complete]).await;
        for request in &requests {
            signed_receipt(request);
        }
        let authorize: Value = serde_json::from_slice(&requests[0].body).expect("authorize JSON");
        let complete: Value = serde_json::from_slice(&requests[1].body).expect("complete JSON");
        assert!(
            authorize
                == json!({"redirect_uri": REDIRECT, "state": "iam-v2-state",
            "nonce": nonce(), "code_challenge": "c".repeat(43), "code_challenge_method": "S256"}),
            "authorize body exact: no verifier, slug or profile selector"
        );
        assert!(
            complete
                == json!({"code": CODE, "redirect_uri": REDIRECT,
            "code_verifier": "V".repeat(64), "expected_nonce": nonce()}),
            "complete body exact"
        );
    }
}

// G-4; mutation: incorrectly require PKCE even when policy disabled it.
#[tokio::test]
async fn v2_without_pkce_still_sends_nonce_and_omits_optional_wire_fields() {
    let server = MockServer::start().await;
    let client = client(&server, "no-pkce");
    let mut authorize = authorize_request();
    authorize.code_challenge = None;
    authorize.code_challenge_method = None;
    let mut complete = complete_request();
    complete.code_verifier = None;
    for operation in [Operation::Authorize, Operation::Complete] {
        mount(
            &server,
            "no-pkce",
            operation,
            ResponseTemplate::new(200).set_body_json(operation.response()),
        )
        .await;
    }
    client
        .authorize_authentication(&authorize)
        .await
        .expect("nonce without PKCE accepted");
    client
        .complete_authorization(&complete)
        .await
        .expect("completion without PKCE accepted");
    let requests = receipts(
        &server,
        "no-pkce",
        &[Operation::Authorize, Operation::Complete],
    )
    .await;
    for request in &requests {
        signed_receipt(request);
    }
    let start: Value = serde_json::from_slice(&requests[0].body).expect("authorize JSON");
    let finish: Value = serde_json::from_slice(&requests[1].body).expect("complete JSON");
    assert!(
        start == json!({"redirect_uri": REDIRECT, "state": "iam-v2-state", "nonce": nonce()}),
        "no optional PKCE fields in authorize"
    );
    assert!(
        finish == json!({"redirect_uri": REDIRECT, "code": CODE, "expected_nonce": nonce()}),
        "no verifier in complete"
    );
}

// G-4/G-9; mutation: syntax-only nonce validation accepts noncanonical trailing bits.
#[tokio::test]
async fn v2_bad_nonce_syntax_length_padding_and_noncanonical_bits_refuse_before_io() {
    let server = MockServer::start().await;
    let client = client(&server, "invalid-nonce");
    let invalid = [
        String::new(),
        "A".repeat(42),
        "A".repeat(44),
        format!("{}=", "A".repeat(42)),
        format!("{}+", "A".repeat(42)),
        format!("{}B", "A".repeat(42)),
        format!("{}\0", "A".repeat(42)),
    ];
    for nonce in invalid {
        let mut start = authorize_request();
        start.nonce.clone_from(&nonce);
        let error = client
            .authorize_authentication(&start)
            .await
            .expect_err("invalid nonce must refuse");
        sanitized(error, Operation::Authorize, &server);
        let mut complete = complete_request();
        complete.expected_nonce = nonce;
        let error = client
            .complete_authorization(&complete)
            .await
            .expect_err("invalid expected nonce must refuse");
        sanitized(error, Operation::Complete, &server);
    }
    receipts(&server, "invalid-nonce", &[]).await;
}

// G-4; mutation: serialize invalid/incomplete PKCE pairs before validating.
#[tokio::test]
async fn v2_authorize_bad_pkce_pairs_refuse_before_io() {
    let server = MockServer::start().await;
    let client = client(&server, "invalid-pkce");
    for (challenge, method) in [
        (Some("c".repeat(43)), None),
        (None, Some("S256")),
        (Some("c".repeat(43)), Some("plain")),
        (Some("c".repeat(42)), Some("S256")),
        (Some(format!("{}+", "c".repeat(42))), Some("S256")),
    ] {
        let mut request = authorize_request();
        request.code_challenge = challenge;
        request.code_challenge_method = method.map(str::to_owned);
        let error = client
            .authorize_authentication(&request)
            .await
            .expect_err("bad PKCE must refuse");
        sanitized(error, Operation::Authorize, &server);
    }
    receipts(&server, "invalid-pkce", &[]).await;
}

// G-4; mutation: completion forwards malformed verifier to Connect.
#[tokio::test]
async fn v2_complete_bad_verifier_refuses_before_io() {
    let server = MockServer::start().await;
    let client = client(&server, "invalid-verifier");
    for verifier in [
        String::new(),
        "V".repeat(42),
        "V".repeat(129),
        format!("{}+", "V".repeat(42)),
    ] {
        let mut request = complete_request();
        request.code_verifier = Some(verifier);
        let error = client
            .complete_authorization(&request)
            .await
            .expect_err("bad verifier must refuse");
        sanitized(error, Operation::Complete, &server);
    }
    receipts(&server, "invalid-verifier", &[]).await;
}

async fn rejected_response(operation: Operation, status: u16, body: &str) {
    let server = MockServer::start().await;
    let client = client(&server, "remote-failure");
    mount(
        &server,
        "remote-failure",
        operation,
        ResponseTemplate::new(status).set_body_raw(body, "application/json"),
    )
    .await;
    let error = call(&client, operation)
        .await
        .expect_err("unusable remote response must refuse");
    sanitized(error, operation, &server);
    receipts(&server, "remote-failure", &[operation]).await;
}

// G-4/G-9; mutation: tokens-only success, parsing errors leak content, fallback/retry.
#[tokio::test]
async fn v2_complete_http_errors_invalid_json_and_partial_identity_are_generic_without_fallback() {
    for status in [401, 403, 500] {
        rejected_response(Operation::Complete, status, REMOTE_ERROR).await;
    }
    let mut missing_profile = Operation::Complete.response();
    missing_profile
        .as_object_mut()
        .expect("fixture object")
        .remove("profile");
    let mut malformed_profile = Operation::Complete.response();
    malformed_profile["profile"]["id"] = json!(42);
    for body in [
        format!("<html>{REMOTE_ERROR}</html>"),
        format!("{{\"tokens\":\"{ACCESS}"),
        missing_profile.to_string(),
        malformed_profile.to_string(),
        "null".into(),
    ] {
        rejected_response(Operation::Complete, 200, &body).await;
    }
}

// G-4/G-9; mutation: authorize accepts HTML/error status or leaks serde source.
#[tokio::test]
async fn v2_authorize_http_and_dto_failures_are_generic_without_retry() {
    for status in [401, 403, 500] {
        rejected_response(Operation::Authorize, status, REMOTE_ERROR).await;
    }
    for body in [
        "null",
        "{",
        REMOTE_ERROR,
        "{\"authorization_url\":42,\"scope\":\"openid\"}",
    ] {
        rejected_response(Operation::Authorize, 200, body).await;
    }
}

fn padded_json(operation: Operation, size: usize) -> String {
    let mut value = operation.response();
    value["unknown_padding"] = json!("");
    let overhead = serde_json::to_vec(&value).expect("fixture JSON").len();
    value["unknown_padding"] =
        json!("p".repeat(size.checked_sub(overhead).expect("padding budget")));
    let body = serde_json::to_string(&value).expect("fixture JSON");
    assert_eq!(body.len(), size, "exact UTF-8 byte boundary");
    let valid_fixture = match operation {
        Operation::Authorize => serde_json::from_str::<AuthorizeResponse>(&body).is_ok(),
        Operation::Complete => serde_json::from_str::<ProviderAuthentication>(&body).is_ok(),
    };
    assert!(
        valid_fixture,
        "padding is an unknown field in a complete, valid response DTO"
    );
    body
}

// G-4; mutation: >= instead of > rejects the contractual inclusive boundary.
#[tokio::test]
async fn v2_authorize_and_complete_accept_valid_json_at_exactly_one_mib() {
    for operation in [Operation::Authorize, Operation::Complete] {
        let server = MockServer::start().await;
        let client = client(&server, "body-boundary");
        mount(
            &server,
            "body-boundary",
            operation,
            ResponseTemplate::new(200)
                .set_body_raw(padded_json(operation, LIMIT), "application/json"),
        )
        .await;
        call(&client, operation)
            .await
            .expect("inclusive 1 MiB JSON must succeed");
        receipts(&server, "body-boundary", &[operation]).await;
    }
}

// G-4; mutation: response.json() unbounded accepts complete valid oversized DTO.
// This observable refusal alone does NOT prove bounded accumulation before parse;
// reader source review and a no-Content-Length witness remain separate evidence.
#[tokio::test]
async fn v2_authorize_and_complete_refuse_valid_json_one_byte_over_one_mib() {
    for operation in [Operation::Authorize, Operation::Complete] {
        rejected_response(operation, 200, &padded_json(operation, LIMIT + 1)).await;
    }
}

// G-4/G-9; mutation: following redirects forwards signed body/credentials.
#[tokio::test]
async fn v2_redirects_do_not_forward_to_another_isolated_plaintext_endpoint() {
    for operation in [Operation::Authorize, Operation::Complete] {
        let original = MockServer::start().await;
        let target = MockServer::start().await;
        let client = client(&original, "redirect-origin");
        let location = format!(
            "{}/oauth-connect/redirect-target{}",
            target.uri(),
            operation.suffix()
        );
        mount(
            &target,
            "redirect-target",
            operation,
            ResponseTemplate::new(200).set_body_json(operation.response()),
        )
        .await;
        mount(
            &original,
            "redirect-origin",
            operation,
            ResponseTemplate::new(307)
                .insert_header("location", location.as_str())
                .set_body_string(REMOTE_ERROR),
        )
        .await;
        let error = call(&client, operation)
            .await
            .expect_err("redirect must refuse");
        sanitized(error, operation, &original);
        receipts(&original, "redirect-origin", &[operation]).await;
        receipts(&target, "redirect-target", &[]).await;
    }
}

// G-4/G-9; mutation: disabled total timeout, immediate refusal, or timeout retry.
#[tokio::test]
async fn v2_complete_real_ten_second_timeout_has_one_request_and_generic_error() {
    let server = MockServer::start().await;
    let client = client(&server, "slow-complete");
    mount(
        &server,
        "slow-complete",
        Operation::Complete,
        ResponseTemplate::new(200)
            .set_body_json(Operation::Complete.response())
            .set_delay(Duration::from_secs(12)),
    )
    .await;
    let started = Instant::now();
    let error = call(&client, Operation::Complete)
        .await
        .expect_err("12s response exceeds real 10s client budget");
    let elapsed = started.elapsed();
    sanitized(error, Operation::Complete, &server);
    receipts(&server, "slow-complete", &[Operation::Complete]).await;
    assert!(
        elapsed >= Duration::from_secs(9),
        "must reach real timeout, not immediate validation refusal"
    );
    assert!(
        elapsed < Duration::from_secs(40),
        "generous scheduling ceiling; no unbounded wait"
    );
}
