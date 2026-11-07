//! C-1/G-3/G-6: local outbound collaborators and closed observable assertions.
//! No client builder lives here: every product request uses the client returned by C.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use idp_connect_contract::{
    sign, verify, CompleteAuthorizationRequest, FederatedOAuthError, HmacKey,
    ProviderAuthentication,
};
use oauth_connect_configuration::{
    AppConfig, OpenBaoAuthConfig, OpenBaoConfig, ProviderConfig, SecretReference, SecretsMode,
};
use rustycog::core::error::DomainError;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Debug, Display},
};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

pub const TOKEN_PATH: &str = "/grant/code";
pub const USER_PATH: &str = "/people/self";
pub const EMAIL_PATH: &str = "/contact/primary";
pub const CLIENT: &str = " \tC-client+&=:/sentinel \r\n";
pub const HMAC_RAW: &str = "\u{2003}C-HMAC interior\tcanonical-sentinel\r\n";
pub const HMAC: &str = "C-HMAC interior\tcanonical-sentinel";
pub const ACCESS: &str = "c-access-sentinel";
pub const REFRESH: &str = "c-refresh-sentinel";
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub const BAO_TOKEN: &str = "bootstrap-c-sentinel";
pub const BAO_LOOKUP: &str = "/v1/auth/token/lookup-self";
pub const BAO_CLIENT: &str = "/v1/secret/data/github-connect/oauth";
pub const BAO_HMAC: &str = "/v1/secret/data/github-connect/iam";

pub fn redacted(text: &str) {
    for marker in ["sentinel", "github-connect/oauth", "github-connect/iam"] {
        assert!(
            !text.to_ascii_lowercase().contains(marker),
            "C: sensitive material in a diagnostic"
        );
    }
}

pub fn must<T, E: Debug + Display>(value: Result<T, E>, phase: &str) -> T {
    value.unwrap_or_else(|error| {
        redacted(&format!("{error:?} {error}"));
        panic!("C: expected successful {phase}");
    })
}

pub fn rejected<T>(value: Result<T, DomainError>) {
    let Err(error) = value else {
        panic!("C: invalid catalogue returned a usable collection")
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
    assert!(source.is_none(), "C: unbounded error source chain");
}

pub fn exchange_error(value: Result<ProviderAuthentication, FederatedOAuthError>) {
    let Err(error) = value else {
        panic!("C: failed exchange returned partial/successful authentication")
    };
    assert_eq!(
        error,
        FederatedOAuthError::ExchangeCode,
        "C: exact existing exchange-error category"
    );
    redacted(&format!("{error:?} {error}"));
}

pub fn callback(id: &str) -> String {
    format!("https://iam.example.invalid/{id}/callback")
}

pub fn request(id: &str) -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        code: "code-c+&=:/sentinel".into(),
        redirect_uri: callback(id),
        code_verifier: Some(VERIFIER.into()),
        expected_nonce: URL_SAFE_NO_PAD.encode([9_u8; 32]),
    }
}

pub fn provider(id: &str, origin: &str, client: &str, hmac: &str) -> ProviderConfig {
    let value = serde_json::from_value(json!({
        "id": id, "client_id": format!("public-{id}"),
        "redirect_uris": [callback(id), format!("https://iam.example.invalid/{id}/relink-callback")],
        "scopes": ["user:email"], "pkce_supported": true, "secrets_mode": "direct",
        "client_secret": client, "hmac_secret": hmac, "token_auth_method": "client_secret_post",
        "transport_security": "isolated_test",
        "profile": {"kind": "oauth", "authorization_url": format!("{origin}/authorize"),
            "token_url": format!("{origin}{TOKEN_PATH}"),
            "apis": {
                "user": {"url": format!("{origin}{USER_PATH}"), "access_token_auth": "bearer"},
                "emails": {"url": format!("{origin}{EMAIL_PATH}"), "access_token_auth": "token"}
            },
            "claims": {
                "sub": {"api": "user", "json_path": "$.id"},
                "preferred_username": {"api": "user", "json_path": "$.login"},
                "picture": {"api": "user", "json_path": "$.avatar_url"},
                "email": {"api": "emails", "json_path": "$[?(@.primary == true)].email"},
                "email_verified": {"api": "emails", "json_path": "$[?(@.primary == true)].verified"}
            }
        }
    }));
    value.unwrap_or_else(|_| panic!("C fixture must match the integrated provider shape"))
}

pub fn config(providers: Vec<ProviderConfig>) -> AppConfig {
    AppConfig {
        providers,
        ..AppConfig::default()
    }
}

pub fn vault_provider(mut p: ProviderConfig) -> ProviderConfig {
    p.secrets_mode = SecretsMode::Openbao;
    p.client_secret = None;
    p.hmac_secret = None;
    p.client_secret_ref = Some(must(
        SecretReference::new(
            "secret".into(),
            "github-connect/oauth".into(),
            "client_secret".into(),
        ),
        "client reference",
    ));
    p.hmac_secret_ref = Some(must(
        SecretReference::new(
            "secret".into(),
            "github-connect/iam".into(),
            "hmac_secret".into(),
        ),
        "HMAC reference",
    ));
    p
}

pub fn tokens(access: &str) -> Value {
    json!({"access_token": access, "refresh_token": REFRESH, "expires_in": 3600, "token_type": "Bearer"})
}

pub fn user(id: u64, login: &str) -> Value {
    json!({"id": id, "login": login, "avatar_url": "https://images.example.invalid/avatar"})
}

pub fn emails(email: &str) -> Value {
    json!([{"email": email, "primary": true, "verified": true}])
}

pub fn authentication(
    value: ProviderAuthentication,
    id: &str,
    login: &str,
    email: &str,
    access: &str,
) {
    redacted(&format!("{value:?}"));
    assert!(
        value.tokens.access_token == access,
        "C: wrong instance access token"
    );
    assert!(
        value.tokens.refresh_token.as_deref() == Some(REFRESH),
        "C: wrong refresh token"
    );
    assert_eq!(value.tokens.expires_in, Some(3600));
    assert_eq!(value.profile.id, id);
    assert_eq!(value.profile.username, login);
    assert!(
        value.profile.email.as_deref() == Some(email),
        "C: wrong email from independent API"
    );
    assert!(value.profile.email_verified);
    assert_eq!(
        value.profile.avatar_url.as_deref(),
        Some("https://images.example.invalid/avatar")
    );
}

pub fn hmac_proof(key: &HmacKey, id: &str, canonical: &str, raw: &str, other: &str) {
    redacted(&format!("{key:?}"));
    let body = must(serde_json::to_string(&request(id)), "v2 body serialization");
    let full_path = format!("/oauth-connect/{id}/v2/complete");
    let timestamp = 1_700_000_000;
    let signature = must(
        sign(key.as_bytes(), "POST", &full_path, timestamp, &body),
        "composed-key signing",
    );
    must(
        verify(
            canonical.as_bytes(),
            "POST",
            &full_path,
            timestamp,
            &body,
            &signature,
            timestamp,
        ),
        "verification with independently provisioned canonical HMAC",
    );
    for wrong in [raw, other] {
        assert!(
            verify(
                wrong.as_bytes(),
                "POST",
                &full_path,
                timestamp,
                &body,
                &signature,
                timestamp
            )
            .is_err(),
            "C: raw/wrong-instance HMAC accepted"
        );
    }
    for wrong_path in ["/v2/complete", "/oauth-connect/another/v2/complete"] {
        assert!(
            verify(
                canonical.as_bytes(),
                "POST",
                wrong_path,
                timestamp,
                &body,
                &signature,
                timestamp
            )
            .is_err(),
            "C: full instance PATH is not bound to the HMAC"
        );
    }
    assert!(
        verify(
            canonical.as_bytes(),
            "POST",
            &full_path,
            timestamp,
            &format!("{body} "),
            &signature,
            timestamp
        )
        .is_err(),
        "C: changed exact body was accepted"
    );
}

pub struct Vendor {
    pub server: MockServer,
}
impl Vendor {
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }
    pub fn origin(&self) -> String {
        self.server.uri()
    }
    pub async fn response(&self, verb: &str, route: &str, response: ResponseTemplate) {
        Mock::given(method(verb))
            .and(path(route))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }
    pub async fn success(&self, id: u64, login: &str, email: &str, access: &str) {
        self.response(
            "POST",
            TOKEN_PATH,
            ResponseTemplate::new(200).set_body_json(tokens(access)),
        )
        .await;
        self.response(
            "GET",
            USER_PATH,
            ResponseTemplate::new(200).set_body_json(user(id, login)),
        )
        .await;
        self.response(
            "GET",
            EMAIL_PATH,
            ResponseTemplate::new(200).set_body_json(emails(email)),
        )
        .await;
    }
    pub async fn receipts(&self) -> Vec<Request> {
        self.server
            .received_requests()
            .await
            .expect("C retained vendor receipts")
    }
    pub async fn counts(&self, token: usize, user: usize, email: usize) {
        let requests = self.receipts().await;
        assert_eq!(
            requests.len(),
            token + user + email,
            "C: no extra endpoint or picture fetch"
        );
        for (verb, route, expected) in [
            ("POST", TOKEN_PATH, token),
            ("GET", USER_PATH, user),
            ("GET", EMAIL_PATH, email),
        ] {
            assert_eq!(
                requests
                    .iter()
                    .filter(|r| r.method.as_str() == verb && r.url.path() == route)
                    .count(),
                expected
            );
        }
    }
    pub async fn credentials(&self, id: &str, secret: &str, access: &str) {
        for received in self.receipts().await {
            assert!(
                received.url.query().is_none(),
                "C: credentials/code/token in URL query"
            );
            assert!(
                received
                    .headers
                    .get(idp_connect_contract::SIGNATURE_HEADER)
                    .is_none(),
                "C: IAM HMAC must not be forwarded to the vendor"
            );
            assert!(
                received.headers.get("X-Vault-Token").is_none(),
                "C: bootstrap token sent to vendor"
            );
            if received.method.as_str() == "POST" {
                let pairs: Vec<(String, String)> = url::form_urlencoded::parse(&received.body)
                    .into_owned()
                    .collect();
                let fields: BTreeMap<String, String> = pairs.iter().cloned().collect();
                assert_eq!(pairs.len(), fields.len(), "C: no duplicate form fields");
                assert_eq!(fields.len(), 6, "C: no extra or leaked form credentials");
                for (field, expected) in [
                    ("client_id", format!("public-{id}")),
                    ("client_secret", secret.to_owned()),
                    ("grant_type", "authorization_code".into()),
                    ("code", request(id).code),
                    ("redirect_uri", callback(id)),
                    ("code_verifier", VERIFIER.into()),
                ] {
                    assert!(
                        fields.get(field).is_some_and(|v| v == &expected),
                        "C: wrong exact credential/form field"
                    );
                }
                assert!(
                    received.headers.get("authorization").is_none(),
                    "C: unexpected Basic retry/fallback"
                );
            } else {
                let scheme = if received.url.path() == USER_PATH {
                    "Bearer"
                } else {
                    "token"
                };
                assert!(
                    received
                        .headers
                        .get("authorization")
                        .is_some_and(|h| h == format!("{scheme} {access}").as_str()),
                    "C: access token leaked across instances/APIs"
                );
                assert!(received.body.is_empty(), "C: unexpected GET body");
            }
        }
    }
}

pub struct Vault {
    pub server: MockServer,
    token: tempfile::NamedTempFile,
}
impl Vault {
    pub async fn start() -> Self {
        let token = tempfile::NamedTempFile::new().expect("C isolated token file");
        std::fs::write(token.path(), BAO_TOKEN).expect("C synthetic credential");
        Self {
            server: MockServer::start().await,
            token,
        }
    }
    pub fn config(&self) -> OpenBaoConfig {
        OpenBaoConfig {
            endpoint: self.server.uri(),
            auth: OpenBaoAuthConfig::TokenFile {
                path: self.token.path().to_owned(),
            },
            allow_loopback_http: true,
            ca_cert_file: None,
        }
    }
    pub async fn response(&self, route: &str, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path(route))
            .and(header("X-Vault-Token", BAO_TOKEN))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }
    pub async fn success(&self, client: &str, hmac: &str) {
        self.response(
            BAO_LOOKUP,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"ttl": 600, "policies": ["connector-read"]}})),
        )
        .await;
        self.response(
            BAO_CLIENT,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"data": {"client_secret": client}}})),
        )
        .await;
        self.response(
            BAO_HMAC,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"data": {"hmac_secret": hmac}}})),
        )
        .await;
    }
    pub async fn count(&self, route: &str) -> usize {
        self.server
            .received_requests()
            .await
            .expect("C vault receipts")
            .iter()
            .filter(|r| r.url.path() == route && r.method.as_str() == "GET")
            .count()
    }
    pub async fn total(&self) -> usize {
        self.server
            .received_requests()
            .await
            .expect("C vault receipts")
            .len()
    }
}
