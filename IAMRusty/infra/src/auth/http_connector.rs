//! HTTP + HMAC adapter for `idp-connect-contract::FederatedOAuthClient`.

use async_trait::async_trait;
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
    /// shorter than 16 bytes after trim, or if the HTTP client cannot be constructed.
    pub fn new(
        base_url: impl Into<String>,
        hmac_secret: impl AsRef<[u8]>,
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
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| DomainError::OAuth2Error("failed to build IdP HTTP client".to_string()))?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
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
        let body = AuthorizeRequest {
            redirect_uri: redirect_uri.to_string(),
            state: state.to_string(),
        };
        self.signed_post(AUTHORIZE_PATH, &body, FederatedOAuthError::Authorize)
            .await
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        let body = TokenRequest {
            code: code.to_string(),
            redirect_uri: redirect_uri.to_string(),
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
        HttpIdpConnector::new("http://127.0.0.1:3000/github-connect", "connector-secret")
            .expect("non-empty secret");
    }
}
