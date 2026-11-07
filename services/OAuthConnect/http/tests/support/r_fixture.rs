//! G-5/G-6 fixtures: actual OAuth adapters, only outbound vendors are `WireMock`.
//! The trusted HTTP client below is fixture-only; setup/Application remain C's scope.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use idp_connect_contract::{
    hmac::{sign, unix_timestamp_secs, HmacKey, SIGNATURE_HEADER, TIMESTAMP_HEADER},
    AuthorizeAuthenticationRequest, CompleteAuthorizationRequest, FederatedOAuthClient,
};
use oauth_connect_configuration::ProviderConfig;
use oauth_connect_http::{AppState, ConnectState, DomainError};
use oauth_connect_infra::OAuthClient;
use rustycog::{
    command::{CommandRegistry, GenericCommandService},
    config::secrets::SecretValue,
    http::UserIdExtractor,
    permission::InMemoryPermissionChecker,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Debug, Display},
    future::Future,
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceExt;
use tracing::instrument::WithSubscriber;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

pub const FORGE_AUTHORIZE: &str = "/oauth-connect/forge/v2/authorize";
pub const FORGE_COMPLETE: &str = "/oauth-connect/forge/v2/complete";
pub const ENTERPRISE_AUTHORIZE: &str = "/oauth-connect/enterprise/v2/authorize";
pub const ENTERPRISE_COMPLETE: &str = "/oauth-connect/enterprise/v2/complete";
pub const KEY_A: &str = "routing-HMAC-forge-sentinel";
pub const KEY_B: &str = "routing-HMAC-enterprise-sentinel";
pub const NONCE: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
pub const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub const CALLBACK: &str = "https://iam.example.invalid/callback";
pub const TOKEN_PATH: &str = "/sessions/exchange";
pub const PROFILE_PATH: &str = "/accounts/current";
pub const BODY_LIMIT: usize = 64 * 1024;

pub fn redacted(text: &str) {
    for marker in [
        "sentinel",
        NONCE,
        VERIFIER,
        "forge@example.test",
        "enterprise@example.test",
        "forgeuser",
        "enterpriseuser",
    ] {
        assert!(
            !text.contains(marker),
            "G-6: sensitive data exposed in an error/diagnostic"
        );
    }
}

pub fn must<T, E: Debug + Display>(result: Result<T, E>, phase: &str) -> T {
    result.unwrap_or_else(|error| {
        redacted(&format!("{error:?} {error}"));
        panic!("routing fixture/nominal failure in {phase}");
    })
}

pub fn invalid_registry(result: &Result<Router, DomainError>) {
    let Err(error) = result else {
        panic!("G-5: invalid registry was published")
    };
    redacted(&format!("{error:?} {error}"));
    let mut source = error.source();
    for _ in 0..8 {
        let Some(cause) = source else {
            return;
        };
        redacted(&format!("{cause:?} {cause}"));
        source = cause.source();
    }
    assert!(source.is_none(), "G-6: unexpected unbounded error chain");
}

pub fn state() -> AppState {
    // Required SDK platform state only. No bearer middleware, DB, queue or FGA is installed.
    let extractor = must(
        UserIdExtractor::from_resolved_secret("inactive-platform-extractor-sentinel"),
        "SDK inactive platform extractor",
    );
    AppState::new(
        Arc::new(GenericCommandService::new(Arc::new(
            CommandRegistry::default(),
        ))),
        extractor,
        Arc::new(InMemoryPermissionChecker::new()),
    )
}

pub fn authorize() -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: CALLBACK.into(),
        state: "browser-state-sentinel".into(),
        code_challenge: Some(CHALLENGE.into()),
        code_challenge_method: Some("S256".into()),
        nonce: NONCE.into(),
    }
}
pub fn complete() -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        code: "opaque-code-sentinel".into(),
        redirect_uri: CALLBACK.into(),
        code_verifier: Some(VERIFIER.into()),
        expected_nonce: NONCE.into(),
    }
}
pub fn authorize_body() -> String {
    must(serde_json::to_string(&authorize()), "authorize body")
}
pub fn complete_body() -> String {
    must(serde_json::to_string(&complete()), "complete body")
}

pub fn raw_request(method: &str, uri: &str, body: &str) -> Request<Body> {
    let request = must(
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned())),
        "raw HTTP request",
    );
    assert_eq!(
        request.uri().to_string(),
        uri,
        "G-5: fixture must preserve the raw URI"
    );
    request
}

pub fn sign_headers(
    request: &mut Request<Body>,
    method: &str,
    full_path: &str,
    body: &str,
    key: &str,
    at: i64,
) {
    let signature = must(
        sign(key.as_bytes(), method, full_path, at, body),
        "independent HMAC signing",
    );
    let timestamp_name: axum::http::HeaderName =
        must(TIMESTAMP_HEADER.parse(), "timestamp header name");
    let signature_name: axum::http::HeaderName =
        must(SIGNATURE_HEADER.parse(), "signature header name");
    request.headers_mut().insert(
        timestamp_name,
        must(at.to_string().parse(), "timestamp header"),
    );
    request
        .headers_mut()
        .insert(signature_name, must(signature.parse(), "signature header"));
}

pub fn signed(method: &str, path: &str, body: &str, key: &str) -> Request<Body> {
    let mut request = raw_request(method, path, body);
    sign_headers(
        &mut request,
        method,
        path,
        body,
        key,
        must(unix_timestamp_secs(), "test timestamp"),
    );
    request
}

pub struct Reply {
    pub status: StatusCode,
    pub body: Vec<u8>,
}
impl Reply {
    pub fn json(&self) -> Value {
        must(serde_json::from_slice(&self.body), "HTTP JSON reply")
    }
    pub fn status(&self, expected: StatusCode) {
        assert_eq!(self.status, expected, "G-5/G-6: exact HTTP status");
    }
    pub fn empty_error(&self, expected: StatusCode) {
        self.status(expected);
        assert!(
            self.body.is_empty(),
            "G-6: existing status-only error must not expose contextual content"
        );
    }
    pub fn redacted(&self) {
        redacted(&String::from_utf8_lossy(&self.body));
    }
}

pub async fn send(router: &Router, request: Request<Body>) -> Reply {
    let response = must(router.clone().oneshot(request).await, "real router oneshot");
    let status = response.status();
    // Tiny responses only. The 64KiB test concerns the incoming request, never an unbounded reader.
    let body = must(
        to_bytes(response.into_body(), 8192).await,
        "bounded test response",
    )
    .to_vec();
    Reply { status, body }
}

pub struct Backend {
    pub server: MockServer,
    pub client: Arc<OAuthClient>,
    pub id: &'static str,
    pub key: &'static str,
    pub access: &'static str,
    number: u64,
    login: &'static str,
    email: &'static str,
}
impl Backend {
    pub async fn start(
        id: &'static str,
        key: &'static str,
        access: &'static str,
        number: u64,
        login: &'static str,
        email: &'static str,
    ) -> Self {
        let server = MockServer::start().await;
        let origin = server.uri();
        let config: ProviderConfig = serde_json::from_value(json!({
            "id": id, "client_id": format!("public-{id}"),
            "redirect_uris": [CALLBACK, "https://iam.example.invalid/relink-callback"],
            "scopes": ["read:user", "user:email"], "pkce_supported": true, "secrets_mode": "direct",
            "client_secret": format!("client-{id}-sentinel"), "hmac_secret": key,
            "token_auth_method": "client_secret_post", "transport_security": "isolated_test",
            "profile": {"kind": "oauth", "authorization_url": format!("{origin}/start"),
                "token_url": format!("{origin}{TOKEN_PATH}"),
                "apis": {"user": {"url": format!("{origin}{PROFILE_PATH}"), "access_token_auth": "bearer"}},
                "claims": {
                    "sub": {"api": "user", "json_path": "$.id"},
                    "email": {"api": "user", "json_path": "$.email"},
                    "email_verified": {"api": "user", "json_path": "$.verified"},
                    "preferred_username": {"api": "user", "json_path": "$.login"},
                    "picture": {"api": "user", "json_path": "$.avatar_url"}
                }}
        })).map_or_else(|_| panic!("routing fixture must match real configuration"), std::convert::identity);
        let http = must(
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(10))
                .connect_timeout(Duration::from_secs(3))
                .gzip(true)
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .retry(reqwest::retry::never())
                .build(),
            "trusted outbound fixture client",
        );
        let secret = must(
            SecretValue::new(format!("client-{id}-sentinel")),
            "SDK direct secret",
        );
        let client = Arc::new(must(
            OAuthClient::new(&config, secret, http),
            "actual proven OAuth constructor",
        ));
        redacted(&format!("{client:?}"));
        let backend = Self {
            server,
            client,
            id,
            key,
            access,
            number,
            login,
            email,
        };
        backend
            .responses(backend.token_response(), backend.profile_response())
            .await;
        backend
    }
    pub fn connect(&self) -> ConnectState {
        ConnectState {
            hmac_key: HmacKey::new(self.key.as_bytes()),
            client: self.client.clone(),
        }
    }
    pub fn token_response(&self) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({"access_token": self.access,
            "refresh_token": format!("refresh-{}-sentinel", self.id), "expires_in": 3600, "token_type": "Bearer"}))
    }
    pub fn profile_response(&self) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({"id": self.number, "login": self.login,
            "email": self.email, "verified": true, "avatar_url": format!("{}/avatar", self.server.uri())}))
    }
    pub async fn responses(&self, token: ResponseTemplate, profile: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path(TOKEN_PATH))
            .respond_with(token)
            .mount(&self.server)
            .await;
        Mock::given(method("GET"))
            .and(path(PROFILE_PATH))
            .respond_with(profile)
            .mount(&self.server)
            .await;
    }
    pub async fn counts(&self, token: usize, profile: usize) {
        let receipts = self
            .server
            .received_requests()
            .await
            .expect("isolated retained vendor receipts");
        assert_eq!(
            receipts.len(),
            token + profile,
            "G-5: no extra vendor endpoint or image fetch"
        );
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.method.as_str() == "POST" && r.url.path() == TOKEN_PATH)
                .count(),
            token
        );
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.method.as_str() == "GET" && r.url.path() == PROFILE_PATH)
                .count(),
            profile
        );
    }
    pub async fn credentials(&self) {
        let requests = self
            .server
            .received_requests()
            .await
            .expect("retained vendor receipts");
        for r in requests {
            assert!(
                r.url.query().is_none(),
                "G-6: credential/token/code in a vendor query"
            );
            assert!(
                r.headers.get(SIGNATURE_HEADER).is_none(),
                "G-6: incoming HMAC must not reach vendor"
            );
            if r.method.as_str() == "POST" {
                let fields: BTreeMap<String, String> =
                    url::form_urlencoded::parse(&r.body).into_owned().collect();
                for (field, expected) in [
                    ("client_id", format!("public-{}", self.id)),
                    ("client_secret", format!("client-{}-sentinel", self.id)),
                    ("code", complete().code),
                    ("code_verifier", VERIFIER.into()),
                    ("redirect_uri", CALLBACK.into()),
                    ("grant_type", "authorization_code".into()),
                ] {
                    assert!(
                        fields.get(field).is_some_and(|v| v == &expected),
                        "G-5/G-6: wrong exact vendor form field"
                    );
                }
                assert_eq!(
                    fields.len(),
                    6,
                    "G-6: nonce/HMAC/extra form fields must not be forwarded"
                );
            } else {
                assert!(
                    r.headers
                        .get("authorization")
                        .is_some_and(|v| v == format!("Bearer {}", self.access).as_str()),
                    "G-5: wrong instance access token"
                );
            }
        }
    }
    pub fn assert_authorize(&self, value: &Value) {
        assert_eq!(
            value.as_object().expect("authorize response object").len(),
            2
        );
        assert!(value["scope"].as_str() == Some("read:user user:email"));
        let url = must(
            url::Url::parse(
                value["authorization_url"]
                    .as_str()
                    .expect("authorize URL string"),
            ),
            "authorize URL parse",
        );
        assert_eq!(
            url.origin(),
            must(url::Url::parse(&self.server.uri()), "vendor origin").origin()
        );
        assert_eq!(url.path(), "/start");
        let fields: BTreeMap<String, String> = url.query_pairs().into_owned().collect();
        assert_eq!(
            fields.len(),
            7,
            "G-6: no nonce/verifier or hidden vendor scope"
        );
        for (field, expected) in [
            ("client_id", format!("public-{}", self.id)),
            ("redirect_uri", CALLBACK.into()),
            ("response_type", "code".into()),
            ("scope", "read:user user:email".into()),
            ("state", authorize().state),
            ("code_challenge", CHALLENGE.into()),
            ("code_challenge_method", "S256".into()),
        ] {
            assert!(
                fields.get(field).is_some_and(|v| v == &expected),
                "G-5: exact authorize parameter for selected client"
            );
        }
    }
    pub fn assert_authentication(&self, value: &Value) {
        assert_eq!(value.as_object().expect("authentication object").len(), 2);
        assert_eq!(value["tokens"].as_object().expect("tokens object").len(), 3);
        assert_eq!(
            value["profile"].as_object().expect("profile object").len(),
            5
        );
        assert!(
            value["tokens"]["access_token"].as_str() == Some(self.access),
            "G-5: exact selected-instance access token"
        );
        assert!(
            value["tokens"]["refresh_token"].as_str()
                == Some(format!("refresh-{}-sentinel", self.id).as_str()),
            "G-5: exact refresh token"
        );
        assert_eq!(value["tokens"]["expires_in"].as_u64(), Some(3600));
        assert!(value["profile"]["id"].as_str() == Some(self.number.to_string().as_str()));
        assert!(value["profile"]["username"].as_str() == Some(self.login));
        assert!(value["profile"]["email"].as_str() == Some(self.email));
        assert_eq!(value["profile"]["email_verified"].as_bool(), Some(true));
        assert!(
            value["profile"]["avatar_url"].as_str()
                == Some(format!("{}/avatar", self.server.uri()).as_str())
        );
    }
}

pub struct Pair {
    pub forge: Backend,
    pub enterprise: Backend,
}
impl Pair {
    pub async fn start() -> Self {
        Self {
            forge: Backend::start(
                "forge",
                KEY_A,
                "access-forge-sentinel",
                71,
                "forgeuser",
                "forge@example.test",
            )
            .await,
            enterprise: Backend::start(
                "enterprise",
                KEY_B,
                "access-enterprise-sentinel",
                902,
                "enterpriseuser",
                "enterprise@example.test",
            )
            .await,
        }
    }
    pub fn entries(&self) -> Vec<(String, ConnectState)> {
        vec![
            ("forge".into(), self.forge.connect()),
            ("enterprise".into(), self.enterprise.connect()),
        ]
    }
    pub async fn zero(&self) {
        self.forge.counts(0, 0).await;
        self.enterprise.counts(0, 0).await;
    }
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Future-scoped capture with a nonempty witness, not a global subscriber/env mutation.
pub async fn captured(future: impl Future<Output = ()> + Send) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    async {
        tracing::debug!("ROUTING_CAPTURE_WITNESS");
        future.await;
    }
    .with_subscriber(subscriber)
    .await;
    let bytes = capture
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let Ok(text) = String::from_utf8(bytes) else {
        panic!("routing capture is not UTF-8")
    };
    assert!(
        text.contains("ROUTING_CAPTURE_WITNESS"),
        "G-6: empty trace capture cannot pass"
    );
    redacted(&text);
}
