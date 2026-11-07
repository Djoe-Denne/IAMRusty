//! OAuth-only adapter boundary. T0 has no vendor I/O or successful operations.

use std::fmt;

use async_trait::async_trait;
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile,
};
use oauth_connect_configuration::ProviderConfig;
use reqwest::Client;
use rustycog::config::secrets::SecretValue;

use crate::DomainError;

/// One OAuth instance with its startup-injected HTTP client and resolved secret.
/// Opaque in T0; no constructor can produce a usable instance yet.
pub struct OAuthClient {
    _private: (),
}

impl fmt::Debug for OAuthClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OAuthClient([REDACTED])")
    }
}

impl OAuthClient {
    /// Construct an OAuth-only adapter, consuming the SDK secret and HTTP client.
    /// The caller supplies a client built by setup; no client is built per request.
    ///
    /// # Errors
    /// Always refuses in T0, including OIDC declarations. Future construction must
    /// validate the OAuth declaration/mapping without contacting a vendor.
    pub fn new(
        _provider: &ProviderConfig,
        _client_secret: SecretValue,
        _http: Client,
    ) -> Result<Self, DomainError> {
        Err(DomainError::InvalidInput {
            message: "OAuthConnect OAuth client construction not implemented".to_owned(),
        })
    }
}

#[async_trait]
impl FederatedOAuthClient for OAuthClient {
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
