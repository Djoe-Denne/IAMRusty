//! HTTP client for Manifesto binding grant snapshots.

use std::time::Duration;

use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use lazaret_configuration::ManifestoServiceConfig;
use lazaret_domain::{BindingGrantSnapshot, BindingGrantSnapshotPort, GrantFetchError};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde::Serialize;
use uuid::Uuid;

/// HTTP adapter for [`BindingGrantSnapshotPort`].
#[derive(Debug, Clone)]
pub struct HttpBindingGrantClient {
    base_url: String,
    bearer_token: String,
    hs256_secret: Option<String>,
    issuer: String,
    audience: String,
    client: Client,
}

#[derive(Debug, Serialize)]
struct GrantSnapshotClaims {
    sub: String,
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    jti: String,
}

impl HttpBindingGrantClient {
    /// Build a client. Empty `bearer_token` omits the Authorization header when
    /// no HS256 secret is configured.
    ///
    /// # Errors
    ///
    /// Returns [`GrantFetchError::Transport`] if the HTTP client cannot be built.
    pub fn new(
        base_url: impl AsRef<str>,
        timeout_seconds: u64,
        bearer_token: impl Into<String>,
    ) -> Result<Self, GrantFetchError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Lazaret/1.0")
            .build()
            .map_err(|error| GrantFetchError::Transport(error.to_string()))?;
        Ok(Self {
            base_url: base_url.as_ref().trim_end_matches('/').to_owned(),
            bearer_token: bearer_token.into(),
            hs256_secret: None,
            issuer: "aiforall-platform".to_owned(),
            audience: "manifesto-bindings".to_owned(),
            client,
        })
    }

    /// Build from typed config. Non-empty `bearer_token` is sent as-is; otherwise a
    /// platform HS256 JWT is signed when `hs256_secret` is present.
    ///
    /// # Errors
    ///
    /// Returns [`GrantFetchError::Transport`] if the HTTP client cannot be built.
    pub fn from_config(config: &ManifestoServiceConfig) -> Result<Self, GrantFetchError> {
        let mut client = Self::new(
            &config.base_url,
            config.timeout_seconds,
            config.bearer_token.clone(),
        )?;
        client.hs256_secret = config
            .hs256_secret
            .as_ref()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        client.issuer = if config.issuer.trim().is_empty() {
            "aiforall-platform".to_owned()
        } else {
            config.issuer.clone()
        };
        client.audience = if config.audience.trim().is_empty() {
            "manifesto-bindings".to_owned()
        } else {
            config.audience.clone()
        };
        Ok(client)
    }

    fn build_headers(&self) -> Result<HeaderMap, GrantFetchError> {
        let mut headers = HeaderMap::new();
        let Some(token) = self.authorization_token()? else {
            return Ok(headers);
        };
        let value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|error| GrantFetchError::Transport(error.to_string()))?;
        headers.insert(AUTHORIZATION, value);
        Ok(headers)
    }

    fn authorization_token(&self) -> Result<Option<String>, GrantFetchError> {
        if !self.bearer_token.is_empty() {
            return Ok(Some(self.bearer_token.clone()));
        }
        let Some(secret) = self.hs256_secret.as_deref() else {
            return Ok(None);
        };
        Ok(Some(sign_grant_snapshot_jwt(
            secret,
            &self.issuer,
            &self.audience,
        )?))
    }
}

fn sign_grant_snapshot_jwt(
    secret: &str,
    issuer: &str,
    audience: &str,
) -> Result<String, GrantFetchError> {
    let now = chrono::Utc::now();
    let claims = GrantSnapshotClaims {
        sub: Uuid::new_v4().to_string(),
        iss: issuer.to_owned(),
        aud: audience.to_owned(),
        exp: (now + chrono::Duration::hours(1)).timestamp(),
        iat: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|error| GrantFetchError::Transport(error.to_string()))
}

#[async_trait]
impl BindingGrantSnapshotPort for HttpBindingGrantClient {
    async fn fetch(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        let mut url = format!(
            "{}/api/projects/{project_id}/bindings/{component_id}",
            self.base_url
        );
        if let Some(principal) = principal {
            url.push_str("?principal=");
            url.push_str(&principal.to_string());
        }
        let headers = self.build_headers()?;
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|error| GrantFetchError::Transport(error.to_string()))?;
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(GrantFetchError::NotFound);
        }
        if !status.is_success() {
            return Err(GrantFetchError::Transport(format!(
                "unexpected status {status}"
            )));
        }
        response
            .json()
            .await
            .map_err(|error| GrantFetchError::Transport(error.to_string()))
    }
}
