//! Refusing OIDC preconditions; no discovery, verification or cache behavior yet.

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile,
};
use oauth_connect_configuration::ProviderConfig;
use reqwest::Client;
use rustycog::config::secrets::SecretValue;
use tokio::time::Instant;

use crate::DomainError;

// Private construction seams for colocated tests; neither is a public clock API.
// Advancing cache time must never advance the UTC time used by the verifier.
type MonotonicNow = Arc<dyn Fn() -> Instant + Send + Sync>;
type UtcNow = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Opaque OIDC instance boundary; T0 cannot construct a usable client.
/// No identity, tokens, keys or correlation state are acquired or retained yet.
pub struct OidcClient {
    _private: (),
}

impl fmt::Debug for OidcClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("OidcClient").finish_non_exhaustive()
    }
}

impl OidcClient {
    /// Consume a declaration, resolved SDK secret and trusted setup HTTP client.
    ///
    /// # Errors
    /// Always refuses in T0, without network I/O or publishing a partial client.
    pub async fn new(
        provider: &ProviderConfig,
        client_secret: SecretValue,
        http: Client,
    ) -> Result<Self, DomainError> {
        Self::new_with_clocks(
            provider,
            client_secret,
            http,
            Arc::new(Instant::now),
            Arc::new(Utc::now),
        )
        .await
    }

    /// Private clock injection point for future colocated cache/claims tests.
    ///
    /// # Errors
    /// Always refuses in T0; neither clock is sampled and no cache is hydrated.
    async fn new_with_clocks(
        _provider: &ProviderConfig,
        _client_secret: SecretValue,
        _http: Client,
        _monotonic_now: MonotonicNow,
        _utc_now: UtcNow,
    ) -> Result<Self, DomainError> {
        // An explicitly completed refusing future, not simulated discovery/I/O.
        std::future::ready(Err(DomainError::InvalidInput {
            message: "OAuthConnect OIDC client construction not implemented".to_owned(),
        }))
        .await
    }
}

#[async_trait]
impl FederatedOAuthClient for OidcClient {
    async fn authorize(
        &self,
        _redirect_uri: &str,
        _state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        Err(FederatedOAuthError::Authorize)
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        Err(FederatedOAuthError::ExchangeCode)
    }

    async fn user_profile(
        &self,
        _access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        Err(FederatedOAuthError::UserProfile)
    }

    async fn authorize_with_pkce(
        &self,
        _redirect_uri: &str,
        _state: &str,
        _code_challenge: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        Err(FederatedOAuthError::Authorize)
    }

    async fn exchange_code_with_pkce(
        &self,
        _code: &str,
        _redirect_uri: &str,
        _code_verifier: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        Err(FederatedOAuthError::ExchangeCode)
    }

    async fn authorize_authentication(
        &self,
        _request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        Err(FederatedOAuthError::Authorize)
    }

    async fn complete_authorization(
        &self,
        _request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        Err(FederatedOAuthError::ExchangeCode)
    }
}

#[cfg(test)]
mod tests;
