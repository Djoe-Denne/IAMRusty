//! HTTP + HMAC adapter for `idp-connect-contract::FederatedOAuthClient`.

use async_trait::async_trait;
use iam_configuration::security::{validate_connector_url, SecurityMode};
use iam_domain::error::DomainError;
use iam_domain::port::service::FederatedOAuthClient;
use idp_connect_contract::dto::{
    AuthorizeRequest, AuthorizeResponse, ProfileRequest, ProviderTokens, ProviderUserProfile,
    TokenRequest, AUTHORIZE_PATH, PROFILE_PATH, TOKEN_PATH,
};
use idp_connect_contract::hmac::{
    sign, unix_timestamp_secs, HmacKey, SIGNATURE_HEADER, TIMESTAMP_HEADER,
};
use idp_connect_contract::FederatedOAuthError;
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

/// Minimum HMAC secret length after trim (test.toml `iam-idp-connect-test-hmac` is 26 bytes).
const MIN_HMAC_SECRET_LEN: usize = 16;

/// Outbound IAM client for a single IdP Connect slug.
pub struct HttpIdpConnector {
    http: reqwest::Client,
    base_url: String,
    hmac_key: HmacKey,
}

impl std::fmt::Debug for HttpIdpConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpIdpConnector")
            .field("base_url", &self.base_url)
            .field("hmac_key", &self.hmac_key)
            .finish()
    }
}

impl HttpIdpConnector {
    /// Build a connector for `base_url` (including service prefix).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::OAuth2Error`] if `hmac_secret` is empty, whitespace-only,
    /// shorter than 16 bytes after trim, the URL is not verified HTTPS,
    /// or the HTTP client cannot be constructed.
    pub fn new(
        base_url: impl Into<String>,
        hmac_secret: impl AsRef<[u8]>,
    ) -> Result<Self, DomainError> {
        Self::with_security_mode(base_url, hmac_secret, SecurityMode::Verified)
    }

    /// Build with an explicitly selected local/test transport exemption.
    /// Certificate verification is never disabled, even with a plaintext exemption.
    ///
    /// # Errors
    /// Rejects unsafe URLs, weak HMAC keys, or client construction failure.
    pub fn with_security_mode(
        base_url: impl Into<String>,
        hmac_secret: impl AsRef<[u8]>,
        mode: SecurityMode,
    ) -> Result<Self, DomainError> {
        Self::with_security_mode_and_roots(base_url, hmac_secret, mode, Vec::new())
    }

    /// Build with additional administrator-trusted public CA certificates.
    ///
    /// Roots extend the default trust store for this client only. HTTPS certificate
    /// and hostname verification remain enabled; redirects remain disabled.
    /// Parse PEM inputs with [`reqwest::Certificate::from_pem`] before calling this
    /// constructor. No externally constructed client or private key is accepted.
    /// `SecurityMode::Verified` still requires HTTPS, including with custom roots.
    ///
    /// # Errors
    /// Rejects unsafe URLs, weak HMAC keys, or client construction failure before I/O.
    pub fn with_security_mode_and_roots(
        base_url: impl Into<String>,
        hmac_secret: impl AsRef<[u8]>,
        mode: SecurityMode,
        additional_roots: Vec<reqwest::Certificate>,
    ) -> Result<Self, DomainError> {
        let secret = hmac_secret.as_ref();
        let trimmed = match std::str::from_utf8(secret) {
            Ok(text) => text.trim().as_bytes().to_vec(),
            Err(_) => secret.to_vec(),
        };
        if trimmed.is_empty() {
            return Err(DomainError::OAuth2Error(
                "hmac_secret must not be empty".to_string(),
            ));
        }
        if trimmed.len() < MIN_HMAC_SECRET_LEN {
            return Err(DomainError::OAuth2Error(format!(
                "hmac_secret must be at least {MIN_HMAC_SECRET_LEN} bytes"
            )));
        }
        let base_url = base_url.into();
        validate_connector_url(&base_url, mode)
            .map_err(|message| DomainError::OAuth2Error(message.to_string()))?;
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none());
        for certificate in additional_roots {
            builder = builder.add_root_certificate(certificate);
        }
        let http = builder
            .build()
            .map_err(|_| DomainError::OAuth2Error("failed to build IdP HTTP client".to_string()))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            hmac_key: HmacKey::new(&trimmed),
        })
    }

    async fn signed_post<B, R>(
        &self,
        path_suffix: &str,
        body: &B,
        on_fail: FederatedOAuthError,
    ) -> Result<R, FederatedOAuthError>
    where
        B: Serialize,
        R: DeserializeOwned,
    {
        let url = format!("{}{path_suffix}", self.base_url);
        let parsed = reqwest::Url::parse(&url).map_err(|_| on_fail)?;
        let path = parsed.path().to_string();
        let body_bytes = serde_json::to_vec(body).map_err(|_| on_fail)?;
        let body_str = std::str::from_utf8(&body_bytes).map_err(|_| on_fail)?;
        let timestamp = unix_timestamp_secs().map_err(|_| on_fail)?;
        let signature = sign(self.hmac_key.as_bytes(), "POST", &path, timestamp, body_str)
            .map_err(|_| on_fail)?;

        let response = self
            .http
            .post(url)
            .header(TIMESTAMP_HEADER, timestamp.to_string())
            .header(SIGNATURE_HEADER, signature)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes)
            .send()
            .await
            .map_err(|_| on_fail)?;

        let status = response.status();
        if status.as_u16() == 401 || !status.is_success() {
            return Err(on_fail);
        }

        response.json::<R>().await.map_err(|_| on_fail)
    }
}

#[async_trait]
impl FederatedOAuthClient for HttpIdpConnector {
    async fn authorize(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.authorize_with_pkce(redirect_uri, state, None).await
    }

    async fn authorize_with_pkce(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        let body = AuthorizeRequest {
            redirect_uri: redirect_uri.to_string(),
            state: state.to_string(),
            code_challenge: code_challenge.map(str::to_owned),
            code_challenge_method: code_challenge.map(|_| "S256".to_owned()),
        };
        body.pkce_challenge()?;
        self.signed_post(AUTHORIZE_PATH, &body, FederatedOAuthError::Authorize)
            .await
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.exchange_code_with_pkce(code, redirect_uri, None).await
    }

    async fn exchange_code_with_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        if code_verifier.is_some_and(|value| !idp_connect_contract::dto::valid_pkce_verifier(value))
        {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        let body = TokenRequest {
            code: code.to_string(),
            redirect_uri: redirect_uri.to_string(),
            code_verifier: code_verifier.map(str::to_owned),
        };
        self.signed_post(TOKEN_PATH, &body, FederatedOAuthError::ExchangeCode)
            .await
    }

    async fn user_profile(
        &self,
        access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        let body = ProfileRequest {
            access_token: access_token.to_string(),
        };
        self.signed_post(PROFILE_PATH, &body, FederatedOAuthError::UserProfile)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    // Pure construction/parsing checks: no socket, fixture, or global trust-store mutation.
    #[test]
    fn additional_roots_constructor_preserves_verified_https_requirement() {
        assert!(HttpIdpConnector::with_security_mode_and_roots(
            "http://connector.example/connect",
            "connector-secret",
            SecurityMode::Verified,
            Vec::new(),
        )
        .is_err());
        assert!(HttpIdpConnector::with_security_mode_and_roots(
            "https://connector.example/connect",
            "connector-secret",
            SecurityMode::Verified,
            Vec::new(),
        )
        .is_ok());
    }

    #[test]
    fn malformed_additional_ca_is_rejected_during_input_parsing() {
        assert!(reqwest::Certificate::from_pem(b"not a public CA certificate").is_err());
        assert!(reqwest::Certificate::from_pem(b"").is_err());
    }

    fn isolated_client(server: &MockServer) -> HttpIdpConnector {
        HttpIdpConnector::with_security_mode(
            format!("{}/connect", server.uri()),
            "connector-secret",
            SecurityMode::IsolatedTest,
        )
        .expect("explicit test transport")
    }

    async fn mount_oauth_responses(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/connect/v1/authorize"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "authorization_url": "https://vendor.example/authorize", "scope": "profile"
            })))
            .expect(1)
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/connect/v1/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "vendor-token", "refresh_token": null, "expires_in": 3600
            })))
            .expect(1)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn pkce_fields_are_forwarded_and_covered_by_hmac() {
        let server = MockServer::start().await;
        mount_oauth_responses(&server).await;
        let client = isolated_client(&server);
        let challenge = "a".repeat(43);
        let verifier = "v".repeat(64);
        client
            .authorize_with_pkce(
                "https://iam.example/callback",
                "opaque-state",
                Some(&challenge),
            )
            .await
            .expect("authorize with PKCE");
        client
            .exchange_code_with_pkce(
                "opaque-code",
                "https://iam.example/callback",
                Some(&verifier),
            )
            .await
            .expect("exchange with PKCE");
        let requests = server.received_requests().await.expect("request capture");
        assert_eq!(requests.len(), 2);
        for request in &requests {
            let timestamp = request
                .headers
                .get(TIMESTAMP_HEADER)
                .expect("timestamp")
                .to_str()
                .expect("timestamp text")
                .parse::<i64>()
                .expect("timestamp number");
            let body = std::str::from_utf8(&request.body).expect("JSON body");
            let expected = sign(
                b"connector-secret",
                "POST",
                request.url.path(),
                timestamp,
                body,
            )
            .expect("HMAC");
            assert_eq!(
                request
                    .headers
                    .get(SIGNATURE_HEADER)
                    .expect("signature")
                    .to_str()
                    .unwrap(),
                expected
            );
            let json: serde_json::Value = serde_json::from_slice(&request.body).expect("JSON");
            if request.url.path().ends_with(AUTHORIZE_PATH) {
                assert_eq!(json["code_challenge"], challenge);
                assert_eq!(json["code_challenge_method"], "S256");
            } else {
                assert_eq!(json["code_verifier"], verifier);
            }
        }
    }

    #[tokio::test]
    async fn legacy_requests_omit_pkce_fields() {
        let server = MockServer::start().await;
        mount_oauth_responses(&server).await;
        let client = isolated_client(&server);
        client
            .authorize("https://iam.example/callback", "opaque-state")
            .await
            .expect("legacy authorize");
        client
            .exchange_code("opaque-code", "https://iam.example/callback")
            .await
            .expect("legacy exchange");
        let requests = server.received_requests().await.expect("request capture");
        assert_eq!(requests.len(), 2);
        for request in requests {
            let json: serde_json::Value = serde_json::from_slice(&request.body).expect("JSON");
            assert!(json.get("code_challenge").is_none());
            assert!(json.get("code_challenge_method").is_none());
            assert!(json.get("code_verifier").is_none());
        }
    }

    #[tokio::test]
    async fn malformed_pkce_fails_before_outbound_request() {
        let server = MockServer::start().await;
        let client = isolated_client(&server);
        for invalid in [
            "".to_owned(),
            "a".repeat(42),
            format!("{}=", "a".repeat(42)),
        ] {
            assert!(matches!(
                client
                    .authorize_with_pkce("https://iam.example/callback", "state", Some(&invalid))
                    .await,
                Err(FederatedOAuthError::Authorize)
            ));
        }
        for invalid in [
            "".to_owned(),
            "v".repeat(42),
            "v".repeat(129),
            format!("{}!", "v".repeat(42)),
        ] {
            assert!(matches!(
                client
                    .exchange_code_with_pkce("code", "https://iam.example/callback", Some(&invalid))
                    .await,
                Err(FederatedOAuthError::ExchangeCode)
            ));
        }
        assert!(server
            .received_requests()
            .await
            .expect("request capture")
            .is_empty());
    }

    #[test]
    fn new_rejects_empty_hmac_secret() {
        let err = HttpIdpConnector::new("http://127.0.0.1:3000/github-connect", "")
            .expect_err("empty secret");
        assert!(matches!(err, DomainError::OAuth2Error(msg) if msg.contains("hmac_secret")));
    }

    #[test]
    fn new_rejects_whitespace_hmac_secret() {
        let err = HttpIdpConnector::new("http://127.0.0.1:3000/github-connect", "  \t\n")
            .expect_err("whitespace secret");
        assert!(matches!(err, DomainError::OAuth2Error(msg) if msg.contains("hmac_secret")));
    }

    #[test]
    fn new_rejects_too_short_hmac_secret() {
        let err = HttpIdpConnector::new("http://127.0.0.1:3000/github-connect", "123456789012345")
            .expect_err("too-short secret");
        assert!(
            matches!(err, DomainError::OAuth2Error(msg) if msg.contains("hmac_secret") && msg.contains("16"))
        );
    }

    #[test]
    fn new_accepts_non_empty_hmac_secret() {
        HttpIdpConnector::new("https://idp.example/github-connect", "connector-secret")
            .expect("non-empty secret");
    }

    #[test]
    fn plaintext_requires_explicit_exception() {
        assert!(
            HttpIdpConnector::new("http://localhost:3000/github-connect", "connector-secret")
                .is_err()
        );
        HttpIdpConnector::with_security_mode(
            "http://localhost:3000/github-connect",
            "connector-secret",
            SecurityMode::IsolatedTest,
        )
        .expect("explicit isolated test policy");
    }

    #[test]
    fn connector_url_credentials_never_appear_in_errors() {
        let sentinel = "sentinel-connector-credential";
        let err = HttpIdpConnector::new(
            format!("https://user:{sentinel}@idp.example/connect"),
            "connector-secret",
        )
        .expect_err("URL credentials rejected");
        assert!(!err.to_string().contains(sentinel));
    }
}
