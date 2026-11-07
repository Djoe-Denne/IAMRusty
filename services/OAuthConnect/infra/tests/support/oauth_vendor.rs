//! A-local outbound fixtures, using the existing SDK isolated listener lifecycle.

use std::{collections::BTreeMap, io::Write, sync::Arc, time::Duration};

use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use flate2::{write::GzEncoder, Compression};
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, CompleteAuthorizationRequest, FederatedOAuthError,
    ProviderAuthentication,
};
use oauth_connect_configuration::{
    validate_config, AccessTokenAuth, AppConfig, ClaimSource, JsonGetApi, OAuthClaims,
    OAuthProfile, ProviderConfig, ProviderProfile, SecretsMode, TokenAuthMethod,
    VendorTransportSecurity,
};
use oauth_connect_infra::OAuthClient;
use rustycog::{config::secrets::SecretValue, testing::wiremock::MockServerFixture};
use serde_json::{json, Value};
use url::Url;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

pub const TOKEN_PATH: &str = "/sessions/exchange";
pub const USER_PATH: &str = "/accounts/current";
pub const EMAIL_PATH: &str = "/directory/addresses";
pub const CALLBACK: &str = "https://app.example.test/callback?flow=login";
pub const RELINK: &str = "https://app.example.test/relink?flow=link";
pub const CLIENT_ID: &str = "public+client/id";
pub const SECRET: &str = " secret-A+&=:/canary ";
pub const CODE: &str = "code-A+&=/%canary";
pub const ACCESS: &str = "access-A-canary";
pub const REFRESH: &str = "refresh-A-canary";
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
pub const STATE: &str = "state-A+&=/%canary";
pub const LIMIT: usize = 1_048_576;
pub const NESTED: &str = "$.items[?($.items[?($.items[?($.items[?(@.ok == true)])])])].id";

pub fn must<T, E>(result: Result<T, E>, obligation: &str) -> T {
    result.map_or_else(
        |_| panic!("OAuth A: {obligation} must succeed (diagnostic deliberately redacted)"),
        std::convert::identity,
    )
}

pub fn source(api: &str, json_path: &str) -> ClaimSource {
    ClaimSource {
        api: api.to_owned(),
        json_path: json_path.to_owned(),
    }
}

pub fn oauth(provider: &mut ProviderConfig) -> &mut OAuthProfile {
    match &mut provider.profile {
        ProviderProfile::Oauth(profile) => profile,
        ProviderProfile::Oidc(_) => panic!("fixture must declare OAuth"),
    }
}

pub fn trusted_client(provider: &ProviderConfig, secret: &str) -> OAuthClient {
    must(
        validate_config(&AppConfig {
            providers: vec![provider.clone()],
            ..AppConfig::default()
        }),
        "fixture declaration preflight (a failure here is NOT the constructor business RED)",
    );
    // Fixture only, not a second production builder. B/C own the real setup proof.
    let http = must(
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
        "explicitly bounded fixture HTTP client",
    );
    let secret = must(SecretValue::new(secret.to_owned()), "synthetic SDK secret");
    must(
        OAuthClient::new(provider, secret, http),
        "nominal OAuth constructor with valid declaration and trusted injected client",
    )
}

pub fn authorize_request() -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: CALLBACK.to_owned(),
        state: STATE.to_owned(),
        code_challenge: Some(CHALLENGE.to_owned()),
        code_challenge_method: Some("S256".to_owned()),
        nonce: nonce(),
    }
}

pub fn complete_request() -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        code: CODE.to_owned(),
        redirect_uri: CALLBACK.to_owned(),
        code_verifier: Some(VERIFIER.to_owned()),
        expected_nonce: nonce(),
    }
}

pub fn nonce() -> String {
    URL_SAFE_NO_PAD.encode([7_u8; 32])
}

pub fn tokens() -> Value {
    json!({"access_token": ACCESS, "token_type": "Bearer", "refresh_token": REFRESH, "expires_in": 3600})
}

pub fn emails() -> Value {
    json!([
        {"email": "public@example.test", "primary": false, "verified": true},
        {"email": "primary@example.test", "primary": true, "verified": false}
    ])
}

pub struct Vendor {
    pub server: Arc<MockServer>,
    fixture: MockServerFixture,
}

impl Vendor {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::isolated().await;
        Self {
            server: fixture.server(),
            fixture,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.server.uri())
    }

    pub async fn reset(&self) {
        self.fixture.reset().await;
    }

    pub async fn mount(&self, verb: &str, route: &str, response: ResponseTemplate) {
        Mock::given(method(verb))
            .and(path(route))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }

    pub async fn requests(&self) -> Vec<Request> {
        self.server.received_requests().await.map_or_else(
            || panic!("fixture request recording must be enabled"),
            std::convert::identity,
        )
    }
}

pub struct Flow {
    pub token: Vendor,
    pub user: Vendor,
    pub emails: Vendor,
    pub config: ProviderConfig,
}

impl Flow {
    pub async fn new() -> Self {
        let token = Vendor::new().await;
        let user = Vendor::new().await;
        let emails = Vendor::new().await;
        let apis = BTreeMap::from([
            (
                "user".to_owned(),
                JsonGetApi {
                    url: user.url(USER_PATH),
                    access_token_auth: AccessTokenAuth::Bearer,
                    headers: BTreeMap::from([
                        ("Accept".to_owned(), "application/json".to_owned()),
                        ("User-Agent".to_owned(), "oauth-a-fixture".to_owned()),
                    ]),
                },
            ),
            (
                "emails".to_owned(),
                JsonGetApi {
                    url: emails.url(EMAIL_PATH),
                    access_token_auth: AccessTokenAuth::Token,
                    headers: BTreeMap::from([(
                        "X-GitHub-Api-Version".to_owned(),
                        "2022-11-28".to_owned(),
                    )]),
                },
            ),
        ]);
        let config = ProviderConfig {
            id: "forge".to_owned(),
            client_id: CLIENT_ID.to_owned(),
            redirect_uris: vec![CALLBACK.to_owned(), RELINK.to_owned()],
            scopes: vec!["read:user".to_owned(), "user:email".to_owned()],
            pkce_supported: true,
            secrets_mode: SecretsMode::Direct,
            client_secret: Some(SECRET.to_owned()),
            hmac_secret: Some("hmac-fixture-not-consumed-A".to_owned()),
            client_secret_ref: None,
            hmac_secret_ref: None,
            transport_security: VendorTransportSecurity::IsolatedTest,
            ca_cert_file: None,
            token_auth_method: TokenAuthMethod::ClientSecretPost,
            profile: ProviderProfile::Oauth(Box::new(OAuthProfile {
                authorization_url: token.url("/authorize/arbitrary?prompt=select+account"),
                token_url: token.url(TOKEN_PATH),
                apis,
                claims: OAuthClaims {
                    sub: source("user", "$.id"),
                    email: source("emails", "$[?(@.primary == true)].email"),
                    email_verified: source("emails", "$[?(@.primary == true)].verified"),
                    preferred_username: source("user", "$.login"),
                    picture: Some(source("user", "$.avatar_url")),
                },
            })),
        };
        let flow = Self {
            token,
            user,
            emails,
            config,
        };
        flow.defaults().await;
        flow
    }

    pub fn user_body(&self) -> Value {
        json!({"id": 42, "login": "octocat", "email": "public@example.test",
            "avatar_url": self.emails.url("/images/must-not-fetch")})
    }

    pub fn client(&self) -> OAuthClient {
        trusted_client(&self.config, SECRET)
    }

    pub async fn defaults(&self) {
        self.token.reset().await;
        self.user.reset().await;
        self.emails.reset().await;
        self.token
            .mount("POST", TOKEN_PATH, json_response(&tokens()))
            .await;
        self.user
            .mount("GET", USER_PATH, json_response(&self.user_body()))
            .await;
        self.emails
            .mount("GET", EMAIL_PATH, json_response(&emails()))
            .await;
    }

    pub async fn counts(&self, token: usize, user: usize, emails: usize) {
        assert_eq!(
            self.token.requests().await.len(),
            token,
            "token-phase requests, no retries/GET token"
        );
        assert_eq!(
            self.user.requests().await.len(),
            user,
            "one user GET, no chaining"
        );
        assert_eq!(
            self.emails.requests().await.len(),
            emails,
            "one emails GET, no picture download"
        );
    }

    pub fn assert_profile(&self, auth: &ProviderAuthentication) {
        assert_eq!(auth.profile.id, "42", "exact projected ID");
        assert_eq!(auth.profile.username, "octocat", "exact username");
        assert!(
            auth.profile.email.as_deref() == Some("primary@example.test"),
            "primary email only"
        );
        assert!(
            !auth.profile.email_verified,
            "secondary verification must not verify primary"
        );
        assert!(
            auth.profile.avatar_url.as_deref()
                == Some(self.emails.url("/images/must-not-fetch").as_str()),
            "avatar stays descriptive"
        );
    }
}

pub fn json_response(value: &Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(value)
}

pub fn padded(value: &Value, size: usize) -> Vec<u8> {
    let mut body = must(serde_json::to_vec(value), "fixture JSON serialization");
    assert!(body.len() <= size, "fixture JSON must fit target size");
    body.resize(size, b' '); // JSON whitespace, NOT a large selected/unselected string.
    assert_eq!(body.len(), size, "exact decompressed fixture byte count");
    body
}

pub fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    must(encoder.write_all(body), "fixture gzip input");
    must(encoder.finish(), "fixture gzip finalization")
}

pub fn body_response(body: Vec<u8>, encoding: Option<&str>) -> ResponseTemplate {
    let response = ResponseTemplate::new(200)
        .set_body_bytes(body)
        .insert_header("content-type", "application/json");
    match encoding {
        Some(value) => response.insert_header("content-encoding", value),
        None => response,
    }
}

pub fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

pub fn fields(bytes: &[u8]) -> BTreeMap<String, String> {
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(bytes).into_owned().collect();
    let unique: BTreeMap<String, String> = pairs.iter().cloned().collect();
    assert_eq!(pairs.len(), unique.len(), "no duplicated OAuth fields");
    unique
}

pub fn assert_redacted(text: &str) {
    let authentication_nonce = nonce();
    for sentinel in [
        SECRET,
        CODE,
        ACCESS,
        REFRESH,
        VERIFIER,
        STATE,
        authentication_nonce.as_str(),
    ] {
        assert!(
            !text.contains(sentinel),
            "sensitive OAuth value must not appear in diagnostic"
        );
    }
}

pub fn failed(result: &Result<ProviderAuthentication, FederatedOAuthError>) {
    match result {
        Ok(_) => panic!("complete must fail closed, not return partial tokens/profile"),
        Err(error) => {
            assert!(
                matches!(
                    error,
                    FederatedOAuthError::ExchangeCode | FederatedOAuthError::UserProfile
                ),
                "completion must use an existing exchange/profile error category"
            );
            assert_redacted(&error.to_string());
            assert_redacted(&format!("{error:?}"));
        }
    }
}

pub fn assert_post(
    request: &Request,
    config: &ProviderConfig,
    secret: &str,
    verifier: Option<&str>,
) {
    assert!(
        request.method.as_str() == "POST" && request.url.path() == TOKEN_PATH,
        "exact token POST endpoint"
    );
    assert!(
        request.url.query().is_none(),
        "no code/secret/verifier in URL query"
    );
    assert!(
        header(request, "accept") == Some("application/json"),
        "token Accept JSON"
    );
    assert!(
        header(request, "content-type")
            .is_some_and(|v| v.starts_with("application/x-www-form-urlencoded")),
        "token body is form encoded"
    );
    let form = fields(&request.body);
    assert!(
        form.get("grant_type").map(String::as_str) == Some("authorization_code"),
        "authorization code grant"
    );
    assert!(
        form.get("code").map(String::as_str) == Some(CODE),
        "exact code decoded once"
    );
    assert!(
        form.get("redirect_uri").map(String::as_str) == Some(CALLBACK),
        "exact callback in token form"
    );
    assert!(
        form.get("code_verifier").map(String::as_str) == verifier,
        "verifier only when supplied"
    );
    assert!(
        !form.contains_key("nonce") && !form.contains_key("expected_nonce"),
        "OAuth sends no OIDC nonce"
    );
    match config.token_auth_method {
        TokenAuthMethod::ClientSecretPost => {
            assert!(
                header(request, "authorization").is_none(),
                "no Basic alongside Post auth"
            );
            assert!(
                form.get("client_id") == Some(&config.client_id),
                "exact Post client ID"
            );
            assert!(
                form.get("client_secret").map(String::as_str) == Some(secret),
                "exact Post secret, no trim"
            );
            assert_eq!(
                form.len(),
                if verifier.is_some() { 6 } else { 5 },
                "only explicit OAuth fields"
            );
        }
        TokenAuthMethod::ClientSecretBasic => {
            assert!(
                !form.contains_key("client_secret"),
                "Basic must not also send the client secret in form"
            );
            if let Some(id) = form.get("client_id") {
                assert!(
                    id == &config.client_id,
                    "optional public client ID remains exact"
                );
            }
            let Some(encoded) =
                header(request, "authorization").and_then(|v| v.strip_prefix("Basic "))
            else {
                panic!("Basic authorization required");
            };
            let decoded = must(STANDARD.decode(encoded), "Basic base64");
            let text = must(String::from_utf8(decoded), "Basic UTF-8");
            let Some((id, password)) = text.split_once(':') else {
                panic!("Basic username/password separator required");
            };
            // RFC6749 §2.3.1: percent/form encoding precedes Basic base64.
            let id = fields(format!("x={id}").as_bytes());
            let password = fields(format!("x={password}").as_bytes());
            assert!(
                id.get("x") == Some(&config.client_id),
                "exact decoded Basic client ID"
            );
            assert!(
                password.get("x").map(String::as_str) == Some(secret),
                "exact decoded Basic secret, no trim"
            );
            let public_id_fields = usize::from(form.contains_key("client_id"));
            let required_fields = if verifier.is_some() { 4 } else { 3 };
            assert_eq!(
                form.len(),
                required_fields + public_id_fields,
                "no unexpected/fallback auth fields"
            );
        }
    }
}

pub fn nested_body(count: usize) -> Value {
    let items: Vec<Value> = (0..count)
        .map(|i| {
            if i == 0 {
                json!({"ok": true, "id": "stable"})
            } else {
                json!({"ok": true})
            }
        })
        .collect();
    json!({"items": items, "p": {"e": "one@example.test", "v": true, "u": "name"}})
}

pub fn nested_config(config: &mut ProviderConfig) {
    let profile = oauth(config);
    profile.apis.remove("emails");
    profile.claims = OAuthClaims {
        sub: source("user", NESTED),
        email: source("user", "$.p.e"),
        email_verified: source("user", "$.p.v"),
        preferred_username: source("user", "$.p.u"),
        picture: None,
    };
}

pub fn parsed_url(text: &str) -> Url {
    must(Url::parse(text), "returned URL parsing")
}
