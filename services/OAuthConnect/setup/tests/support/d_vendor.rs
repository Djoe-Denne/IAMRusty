//! G-7/G-8 local wire data, adapted from the integrated OIDC/RSA and OAuth fixtures.
//! Each `MockServer` is isolated on port0; no process-global mock or real provider.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use idp_connect_contract::{
    hmac::{sign, unix_timestamp_secs},
    AuthorizeAuthenticationRequest, CompleteAuthorizationRequest,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use oauth_connect_configuration::{AppConfig, ProviderConfig, ServerConfig};
use rsa::{pkcs8::DecodePrivateKey, traits::PublicKeyParts, RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    sync::OnceLock,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

pub const CALLBACK: &str = "https://iam.example.test/connect/callback";
pub const NONCE: &str = "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk";
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
pub const KEY_OAUTH: &str = "oauth-service-key-sentinel";
pub const KEY_OIDC: &str = "oidc-service-key-sentinel";
pub const RAW_KEY: &str = " \toidc-service-key-sentinel\r\n";
pub const SECRET_OIDC: &str = " oidc-client+&=:/sentinel ";
pub const DISCOVERY: &str = "/.well-known/openid-configuration";

pub fn clean(text: &str) {
    for sensitive in [
        "sentinel",
        NONCE,
        VERIFIER,
        "PRIVATE KEY",
        "one@example.test",
    ] {
        assert!(
            !text.contains(sensitive),
            "G-7/G-8: confidential content in diagnostic"
        );
    }
}
pub fn must<T, E: Debug + Display>(result: Result<T, E>, phase: &str) -> T {
    result.unwrap_or_else(|error| {
        clean(&format!("{error:?} {error}"));
        panic!("G-7/G-8 expected successful {phase}");
    })
}
pub fn authorize() -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: CALLBACK.into(),
        state: "service-state-sentinel".into(),
        code_challenge: Some(CHALLENGE.into()),
        code_challenge_method: Some("S256".into()),
        nonce: NONCE.into(),
    }
}
pub fn complete() -> CompleteAuthorizationRequest {
    CompleteAuthorizationRequest {
        code: "service-code-sentinel".into(),
        redirect_uri: CALLBACK.into(),
        code_verifier: Some(VERIFIER.into()),
        expected_nonce: NONCE.into(),
    }
}
pub fn body(complete_flow: bool) -> String {
    if complete_flow {
        must(
            serde_json::to_string(&complete()),
            "v2 complete serialization",
        )
    } else {
        must(
            serde_json::to_string(&authorize()),
            "v2 authorize serialization",
        )
    }
}
pub fn signature(path: &str, body: &str, key: &str) -> (i64, String) {
    let at = must(unix_timestamp_secs(), "independent HMAC timestamp");
    (
        at,
        must(
            sign(key.as_bytes(), "POST", path, at, body),
            "independent full-path signature",
        ),
    )
}

const fn pem() -> &'static str {
    include_str!("../../../../IAMRusty/config/keys/test-platform.pem")
}
pub fn jwks() -> Value {
    static PUBLIC: OnceLock<Value> = OnceLock::new();
    PUBLIC.get_or_init(|| {
        let private = must(RsaPrivateKey::from_pkcs8_pem(pem()), "existing synthetic RSA key");
        let public = RsaPublicKey::from(&private);
        json!({"keys": [{"kty": "RSA", "use": "sig", "alg": "RS256", "kid": "service-key",
            "n": URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()), "e": URL_SAFE_NO_PAD.encode(public.e().to_bytes_be())}]})
    }).clone()
}
pub fn claims(issuer: &str) -> Value {
    let now = Utc::now().timestamp();
    json!({"iss": issuer, "sub": "000042", "aud": "public-authority", "exp": now + 3600,
        "iat": now - 10, "nonce": NONCE, "email": "one@example.test", "email_verified": true,
        "preferred_username": "name", "picture": format!("{issuer}/avatar")})
}
pub fn token(issuer: &str) -> Value {
    token_with_claims(&claims(issuer))
}
pub fn token_with_claims(claims: &Value) -> Value {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("service-key".into());
    let encoding = must(
        EncodingKey::from_rsa_pem(pem().as_bytes()),
        "existing RSA signing fixture",
    );
    let jwt = must(
        jsonwebtoken::encode(&header, claims, &encoding),
        "fixture ID token signing",
    );
    json!({"access_token": "oidc-access-sentinel", "refresh_token": "oidc-refresh-sentinel",
        "expires_in": 3600, "token_type": "Bearer", "id_token": jwt})
}
pub fn metadata(issuer: &str) -> Value {
    json!({"issuer": issuer, "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"), "jwks_uri": format!("{issuer}/keys"),
        "response_types_supported": ["code"], "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "token_endpoint_auth_methods_supported": ["client_secret_post"],
        "scopes_supported": ["openid", "profile", "email"], "code_challenge_methods_supported": ["S256"]})
}
pub async fn mount(server: &MockServer, verb: &str, route: &str, response: ResponseTemplate) {
    Mock::given(method(verb))
        .and(path(route))
        .respond_with(response)
        .mount(server)
        .await;
}
pub struct Vendors {
    pub oauth: MockServer,
    pub oidc: MockServer,
}
impl Vendors {
    pub async fn new() -> Self {
        let result = Self {
            oauth: MockServer::start().await,
            oidc: MockServer::start().await,
        };
        result.mount_all().await;
        result
    }
    pub async fn mount_all(&self) {
        mount(
            &self.oauth,
            "POST",
            "/token",
            ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "oauth-access-sentinel", "refresh_token": "oauth-refresh-sentinel",
            "expires_in": 3600, "token_type": "Bearer"})),
        )
        .await;
        mount(
            &self.oauth,
            "GET",
            "/profile",
            ResponseTemplate::new(200).set_body_json(json!({
            "id": 71, "login": "forgeuser", "email": "forge@example.test", "verified": true,
            "avatar": format!("{}/avatar", self.oauth.uri())})),
        )
        .await;
        let issuer = self.oidc.uri();
        mount(
            &self.oidc,
            "GET",
            DISCOVERY,
            ResponseTemplate::new(200).set_body_json(metadata(&issuer)),
        )
        .await;
        mount(
            &self.oidc,
            "GET",
            "/keys",
            ResponseTemplate::new(200).set_body_json(jwks()),
        )
        .await;
        mount(
            &self.oidc,
            "POST",
            "/token",
            ResponseTemplate::new(200).set_body_json(token(&issuer)),
        )
        .await;
    }
    pub fn config(&self) -> AppConfig {
        let oauth = self.oauth.uri();
        let oidc = self.oidc.uri();
        let declarations = json!([
            {"id": "forge", "client_id": "public-forge", "redirect_uris": [CALLBACK],
             "scopes": ["user:email"], "pkce_supported": true, "secrets_mode": "direct",
             "client_secret": " oauth-client+&=:/sentinel ", "hmac_secret": KEY_OAUTH,
             "transport_security": "isolated_test", "token_auth_method": "client_secret_post",
             "profile": {"kind": "oauth", "authorization_url": format!("{oauth}/authorize"),
                "token_url": format!("{oauth}/token"), "apis": {"user": {"url": format!("{oauth}/profile"), "access_token_auth": "bearer"}},
                "claims": {"sub": {"api": "user", "json_path": "$.id"},
                    "preferred_username": {"api": "user", "json_path": "$.login"},
                    "email": {"api": "user", "json_path": "$.email"},
                    "email_verified": {"api": "user", "json_path": "$.verified"},
                    "picture": {"api": "user", "json_path": "$.avatar"}}}},
            {"id": "authority", "client_id": "public-authority", "redirect_uris": [CALLBACK],
             "scopes": ["openid", "profile", "email"], "pkce_supported": true, "secrets_mode": "direct",
             "client_secret": SECRET_OIDC, "hmac_secret": RAW_KEY, "transport_security": "isolated_test",
             "token_auth_method": "client_secret_post", "profile": {"kind": "oidc", "issuer": oidc,
                "allowed_endpoint_origins": [oidc]}}
        ]);
        let providers: Vec<ProviderConfig> = must(
            serde_json::from_value(declarations),
            "existing provider configuration shape",
        );
        AppConfig {
            providers,
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                tls_enabled: false,
                ..ServerConfig::default()
            },
            ..AppConfig::default()
        }
    }
    pub async fn requests(&self, oidc: bool) -> Vec<Request> {
        let server = if oidc { &self.oidc } else { &self.oauth };
        server
            .received_requests()
            .await
            .expect("isolated vendor receipts required")
    }
    pub async fn count(&self, oidc: bool, route: &str) -> usize {
        self.requests(oidc)
            .await
            .iter()
            .filter(|r| r.url.path() == route)
            .count()
    }
    pub async fn credentials(&self, oidc: bool) {
        let expected_secret = if oidc {
            SECRET_OIDC
        } else {
            " oauth-client+&=:/sentinel "
        };
        let expected_id = if oidc {
            "public-authority"
        } else {
            "public-forge"
        };
        let receipts = self.requests(oidc).await;
        for r in &receipts {
            assert!(
                r.url.query().is_none(),
                "no credentials or context in vendor query"
            );
            assert!(
                r.headers.get("x-idp-connect-signature").is_none(),
                "IAM HMAC must not reach vendor"
            );
            assert!(
                r.headers.get("x-vault-token").is_none(),
                "bootstrap identity must not reach vendor"
            );
            if r.url.path() == "/profile" {
                assert!(
                    r.headers
                        .get("authorization")
                        .is_some_and(|v| v == "Bearer oauth-access-sentinel"),
                    "profile request must use only selected OAuth token"
                );
            }
        }
        let tokens: Vec<_> = receipts
            .iter()
            .filter(|r| r.url.path() == "/token")
            .collect();
        assert!(
            !tokens.is_empty(),
            "credential proof must reach the vendor token endpoint"
        );
        for r in tokens {
            let pairs: Vec<_> = url::form_urlencoded::parse(&r.body).into_owned().collect();
            let fields: BTreeMap<_, _> = pairs.iter().cloned().collect();
            assert_eq!(pairs.len(), fields.len(), "duplicate token form fields");
            for (name, expected) in [
                ("client_secret", expected_secret),
                ("client_id", expected_id),
                ("code", "service-code-sentinel"),
                ("redirect_uri", CALLBACK),
                ("grant_type", "authorization_code"),
                ("code_verifier", VERIFIER),
            ] {
                assert!(
                    fields.get(name).is_some_and(|v| v == expected),
                    "wrong exact composed vendor form field"
                );
            }
            assert_eq!(
                fields.len(),
                6,
                "nonce/HMAC must not be forwarded to token endpoint"
            );
            assert!(r.url.query().is_none());
            assert!(r.headers.get("x-idp-connect-signature").is_none());
        }
    }
    pub fn assert_profile(&self, value: &Value, oidc: bool) {
        let (access, refresh, id, username, email, origin) = if oidc {
            (
                "oidc-access-sentinel",
                "oidc-refresh-sentinel",
                "000042",
                "name",
                "one@example.test",
                self.oidc.uri(),
            )
        } else {
            (
                "oauth-access-sentinel",
                "oauth-refresh-sentinel",
                "71",
                "forgeuser",
                "forge@example.test",
                self.oauth.uri(),
            )
        };
        assert_eq!(value.as_object().expect("authentication DTO").len(), 2);
        assert_eq!(value["tokens"].as_object().expect("token DTO").len(), 3);
        assert_eq!(value["profile"].as_object().expect("profile DTO").len(), 5);
        assert!(
            value["tokens"]["access_token"].as_str() == Some(access),
            "wrong selected access token"
        );
        assert!(
            value["tokens"]["refresh_token"].as_str() == Some(refresh),
            "wrong selected refresh token"
        );
        assert_eq!(value["tokens"]["expires_in"].as_u64(), Some(3600));
        assert!(value["profile"]["id"].as_str() == Some(id));
        assert!(value["profile"]["username"].as_str() == Some(username));
        assert!(value["profile"]["email"].as_str() == Some(email));
        assert_eq!(value["profile"]["email_verified"].as_bool(), Some(true));
        assert!(
            value["profile"]["avatar_url"].as_str() == Some(format!("{origin}/avatar").as_str())
        );
    }
}

pub fn source(config: &AppConfig) -> String {
    let mut providers = Vec::new();
    for p in &config.providers {
        let mut value = must(
            serde_json::to_value(p),
            "fixture public provider serialization",
        );
        let object = value.as_object_mut().expect("provider object");
        object.retain(|_, v| !v.is_null());
        if let Some(secret) = &p.client_secret {
            object.insert("client_secret".into(), json!(secret));
        }
        if let Some(key) = &p.hmac_secret {
            object.insert("hmac_secret".into(), json!(key));
        }
        providers.push(value);
    }
    // Only direct providers use the file-loader witness; the SDK vault resolver is tested separately.
    must(
        toml::to_string(&json!({"providers": providers, "server": config.server,
        "logging": {"level": "info"}})),
        "real loader TOML input",
    )
}
