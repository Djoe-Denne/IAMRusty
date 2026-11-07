//! Shared test-local OIDC documents, real signatures and isolated HTTP receipts.

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock, PoisonError,
    },
    time::Duration,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use idp_connect_contract::{
    CompleteAuthorizationRequest, FederatedOAuthError, ProviderAuthentication,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use oauth_connect_configuration::{
    OidcProfile, ProviderConfig, ProviderProfile, SecretsMode, TokenAuthMethod,
    VendorTransportSecurity,
};
use rand::{rngs::StdRng, SeedableRng};
use rsa::{
    pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding},
    traits::PublicKeyParts,
    RsaPrivateKey, RsaPublicKey,
};
use rustycog::{config::secrets::SecretValue, testing::wiremock::MockServerFixture};
use serde_json::{json, Value};
use tokio::sync::Notify;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, Respond, ResponseTemplate,
};

pub const DISCOVERY: &str = "/.well-known/openid-configuration";
pub const JWKS: &str = "/keys";
pub const TOKEN: &str = "/token";
pub const USERINFO: &str = "/userinfo";
pub const CALLBACK: &str = "https://app.example.test/oidc/callback?flow=login";
pub const CLIENT_ID: &str = "oidc-client";
pub const SECRET: &str = "oidc-client-secret-canary-32-bytes-minimum";
pub const ACCESS: &str = "oidc-access-token-canary";
pub const REFRESH: &str = "oidc-refresh-token-canary";
pub const SUBJECT: &str = "000042";
pub const KID: &str = "shared-kid";
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

pub fn must<T, E>(result: Result<T, E>, stage: &str) -> T {
    result.map_or_else(
        |_| panic!("OIDC: {stage} must succeed; diagnostic redacted"),
        std::convert::identity,
    )
}

pub fn nonce() -> String {
    URL_SAFE_NO_PAD.encode([9_u8; 32])
}

pub fn secret() -> SecretValue {
    must(SecretValue::new(SECRET.to_owned()), "synthetic SDK secret")
}

pub fn http() -> reqwest::Client {
    // Trusted injected fixture client, not a substitute for C's real builder proof.
    must(
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .gzip(true)
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build(),
        "fixture HTTP client",
    )
}

pub fn completion(code: &str) -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        redirect_uri: CALLBACK.to_owned(),
        code: code.to_owned(),
        code_verifier: Some(VERIFIER.to_owned()),
        expected_nonce: nonce(),
    }
}

pub fn redacted(text: &str) {
    let nonce = nonce();
    for value in [
        SECRET,
        ACCESS,
        REFRESH,
        VERIFIER,
        nonce.as_str(),
        "oidc-code-",
        "eyJ",
    ] {
        assert!(
            !text.contains(value),
            "diagnostic must not expose credentials, nonce, code or JWT"
        );
    }
}

pub fn refused<T, E: std::fmt::Debug + std::fmt::Display>(result: &Result<T, E>, reason: &str) {
    let Err(error) = result else {
        panic!("OIDC: expected closed refusal for {reason}");
    };
    redacted(&error.to_string());
    redacted(&format!("{error:?}"));
}

pub fn authentication_failed(result: &Result<ProviderAuthentication, FederatedOAuthError>) {
    refused(result, "invalid authentication without partial tokens");
    assert!(
        matches!(
            result,
            Err(FederatedOAuthError::ExchangeCode | FederatedOAuthError::UserProfile)
        ),
        "use existing exchange/profile category"
    );
}

/// Synthetic RSA material only, cached once per test binary; never printed.
pub struct TestKey {
    pub encoding: EncodingKey,
    jwk: Value,
}

fn from_private(private: &RsaPrivateKey) -> TestKey {
    let pem = must(
        private.to_pkcs8_pem(LineEnding::LF),
        "synthetic private PEM",
    );
    let public = RsaPublicKey::from(private);
    TestKey {
        encoding: must(EncodingKey::from_rsa_pem(pem.as_bytes()), "RSA encoder"),
        jwk: json!({"kty": "RSA", "use": "sig", "alg": "RS256", "kid": KID,
            "n": URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()),
            "e": URL_SAFE_NO_PAD.encode(public.e().to_bytes_be())}),
    }
}

pub fn key_a() -> &'static TestKey {
    static KEY: OnceLock<TestKey> = OnceLock::new();
    KEY.get_or_init(|| {
        let private = must(
            RsaPrivateKey::from_pkcs8_pem(include_str!(
                "../../../../IAMRusty/config/keys/test-platform.pem"
            )),
            "existing IAM synthetic RSA fixture",
        );
        from_private(&private)
    })
}

pub fn key_b() -> &'static TestKey {
    static KEY: OnceLock<TestKey> = OnceLock::new();
    KEY.get_or_init(|| {
        let private = must(
            RsaPrivateKey::new(&mut StdRng::seed_from_u64(0x01DC_2026), 2048),
            "second deterministic synthetic RSA fixture",
        );
        from_private(&private)
    })
}

pub fn jwk(key: &TestKey, kid: &str) -> Value {
    let mut value = key.jwk.clone();
    value["kid"] = json!(kid);
    value
}

pub fn jwks(key: &TestKey, kid: &str) -> Value {
    json!({"keys": [jwk(key, kid)]})
}

pub fn sign(key: &TestKey, kid: &str, claims: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_owned());
    must(
        jsonwebtoken::encode(&header, claims, &key.encoding),
        "RS256 signed fixture",
    )
}

#[derive(Clone)]
pub struct Document {
    response: Arc<Mutex<ResponseTemplate>>,
    count: Arc<AtomicUsize>,
    entered: Arc<Notify>,
}

impl Document {
    fn new(response: ResponseTemplate) -> Self {
        Self {
            response: Arc::new(Mutex::new(response)),
            count: Arc::new(AtomicUsize::new(0)),
            entered: Arc::new(Notify::new()),
        }
    }
    pub fn set(&self, response: ResponseTemplate) {
        *self.response.lock().unwrap_or_else(PoisonError::into_inner) = response;
    }
    pub fn json(&self, value: &Value) {
        self.set(json_response(value));
    }
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
    pub async fn wait_count(&self, expected: usize) {
        let waited = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let entered = self.entered.notified();
                if self.count() >= expected {
                    break;
                }
                entered.await;
            }
        })
        .await;
        assert!(
            waited.is_ok(),
            "fixture endpoint phase must be reached before watchdog"
        );
    }
}

impl Respond for Document {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.response
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

pub struct OidcFixture {
    pub server: Arc<MockServer>,
    _fixture: MockServerFixture,
    pub provider: ProviderConfig,
    pub discovery: Document,
    pub keys: Document,
    pub token: Document,
    pub userinfo: Document,
}

impl OidcFixture {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::isolated().await;
        let server = fixture.server();
        let issuer = server.uri();
        let provider = ProviderConfig {
            id: "authority".to_owned(),
            client_id: CLIENT_ID.to_owned(),
            redirect_uris: vec![CALLBACK.to_owned()],
            scopes: vec![
                "openid".to_owned(),
                "profile".to_owned(),
                "email".to_owned(),
            ],
            pkce_supported: true,
            secrets_mode: SecretsMode::Direct,
            client_secret: Some(SECRET.to_owned()),
            hmac_secret: Some("oidc-hmac-not-used-by-infra".to_owned()),
            client_secret_ref: None,
            hmac_secret_ref: None,
            ca_cert_file: None,
            transport_security: VendorTransportSecurity::IsolatedTest,
            token_auth_method: TokenAuthMethod::ClientSecretPost,
            profile: ProviderProfile::Oidc(OidcProfile {
                issuer: issuer.clone(),
                allowed_endpoint_origins: vec![issuer],
            }),
        };
        let discovery = Document::new(json_response(&json!({})));
        let keys = Document::new(json_response(&jwks(key_a(), KID)));
        let token = Document::new(json_response(&json!({})));
        let userinfo = Document::new(json_response(
            &json!({"sub": SUBJECT, "email": "one@example.test", "email_verified": true}),
        ));
        let result = Self {
            server,
            _fixture: fixture,
            provider,
            discovery,
            keys,
            token,
            userinfo,
        };
        result.discovery.json(&result.metadata());
        result.set_claims(&result.claims_at(Utc::now()));
        for (verb, route, response) in [
            ("GET", DISCOVERY, result.discovery.clone()),
            ("GET", JWKS, result.keys.clone()),
            ("POST", TOKEN, result.token.clone()),
            ("GET", USERINFO, result.userinfo.clone()),
        ] {
            Mock::given(method(verb))
                .and(path(route))
                .respond_with(response)
                .mount(&result.server)
                .await;
        }
        result
    }
    pub fn url(&self, route: &str) -> String {
        format!("{}{route}", self.server.uri())
    }
    pub fn issuer(&self) -> String {
        self.server.uri()
    }
    pub fn metadata(&self) -> Value {
        json!({"issuer": self.issuer(), "authorization_endpoint": self.url("/authorize"),
            "token_endpoint": self.url(TOKEN), "jwks_uri": self.url(JWKS), "userinfo_endpoint": self.url(USERINFO),
            "response_types_supported": ["code"], "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"], "userinfo_signing_alg_values_supported": ["RS256"],
            "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
            "scopes_supported": ["openid", "profile", "email"], "code_challenge_methods_supported": ["S256"]})
    }
    pub fn claims_at(&self, utc: DateTime<Utc>) -> Value {
        json!({"iss": self.issuer(), "sub": SUBJECT, "aud": CLIENT_ID, "exp": utc.timestamp()+3600,
            "iat": utc.timestamp()-10, "nonce": nonce(), "email": "one@example.test", "email_verified": true,
            "preferred_username": "name", "picture": self.url("/avatar")})
    }
    pub fn token_body(id_token: &str) -> Value {
        json!({"access_token": ACCESS, "refresh_token": REFRESH, "token_type": "Bearer", "expires_in": 3600, "id_token": id_token})
    }
    pub fn set_claims(&self, claims: &Value) {
        self.set_signed(key_a(), KID, claims);
    }
    pub fn set_signed(&self, key: &TestKey, kid: &str, claims: &Value) {
        self.token.json(&Self::token_body(&sign(key, kid, claims)));
    }
    pub async fn receipts(&self) -> Vec<Request> {
        self.server.received_requests().await.map_or_else(
            || panic!("fixture journal required"),
            std::convert::identity,
        )
    }
    pub async fn assert_counts(
        &self,
        discovery: usize,
        keys: usize,
        token: usize,
        userinfo: usize,
    ) {
        assert_eq!(
            (
                self.discovery.count(),
                self.keys.count(),
                self.token.count(),
                self.userinfo.count()
            ),
            (discovery, keys, token, userinfo),
            "exact per-endpoint/phase counts"
        );
        assert_eq!(
            self.receipts().await.len(),
            discovery + keys + token + userinfo,
            "no unconfigured endpoint/picture fetch"
        );
    }
}

pub fn json_response(value: &Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(value)
}

pub fn profile(
    auth: &ProviderAuthentication,
    sub: &str,
    name: &str,
    email: Option<&str>,
    verified: bool,
    picture: Option<&str>,
) {
    assert!(auth.profile.id == sub, "exact non-reinterpreted subject");
    assert!(auth.profile.username == name, "exact descriptive username");
    assert!(
        auth.profile.email.as_deref() == email,
        "email from selected source"
    );
    assert_eq!(
        auth.profile.email_verified, verified,
        "verification from email source"
    );
    assert!(
        auth.profile.avatar_url.as_deref() == picture,
        "picture is descriptive"
    );
    assert!(
        auth.tokens.access_token == ACCESS && auth.tokens.refresh_token.as_deref() == Some(REFRESH),
        "exact returned tokens"
    );
    assert_eq!(
        auth.tokens.expires_in,
        Some(3600),
        "unsigned token lifetime preserved"
    );
    let serialized = must(serde_json::to_value(auth), "public DTO serialization");
    let object = serialized
        .as_object()
        .map_or_else(|| panic!("DTO must be object"), std::convert::identity);
    assert!(
        object.len() == 2 && object.contains_key("tokens") && object.contains_key("profile"),
        "no ID token/raw claims/handle"
    );
    let projected = serialized["profile"]
        .as_object()
        .map_or_else(|| panic!("profile object required"), std::convert::identity);
    assert!(
        projected.len() == 5
            && ["id", "username", "email", "email_verified", "avatar_url"]
                .iter()
                .all(|k| projected.contains_key(*k)),
        "only fixed profile fields"
    );
    redacted(&format!("{auth:?}"));
}

pub fn form(request: &Request) -> BTreeMap<String, String> {
    let pairs: Vec<_> = url::form_urlencoded::parse(&request.body)
        .into_owned()
        .collect();
    let fields: BTreeMap<_, _> = pairs.iter().cloned().collect();
    assert_eq!(pairs.len(), fields.len(), "no duplicate form fields");
    fields
}
