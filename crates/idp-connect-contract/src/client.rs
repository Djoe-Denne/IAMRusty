//! Object-safe federated OAuth port (no vendor HTTP client).

use async_trait::async_trait;

use crate::dto::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    ProviderAuthentication, ProviderTokens, ProviderUserProfile,
};
use crate::error::FederatedOAuthError;

/// Federated authenticator used over HTTP JSON (`/v1/authorize|token|profile`).
///
/// Login vs relink, JWT issuance, and password flows stay in IAM — they are
/// not part of this trait. `provider_id` is a string slug chosen by the
/// connector deployment, not the IAM `Provider` enum.
#[async_trait]
pub trait FederatedOAuthClient: Send + Sync {
    /// Build the vendor authorization URL for the given redirect and state.
    ///
    /// # Errors
    ///
    /// Returns [`FederatedOAuthError::Authorize`] if the URL cannot be built.
    async fn authorize(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError>;

    /// Exchange an authorization code for provider tokens.
    ///
    /// # Errors
    ///
    /// Returns [`FederatedOAuthError::ExchangeCode`] if the exchange fails.
    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError>;

    /// S256 authorize. Legacy adapters must refuse a requested challenge, not downgrade.
    ///
    /// # Errors
    /// Returns [`FederatedOAuthError::Authorize`] if PKCE is unsupported or authorize fails.
    async fn authorize_with_pkce(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        if code_challenge.is_some() {
            return Err(FederatedOAuthError::Authorize);
        }
        self.authorize(redirect_uri, state).await
    }

    /// Forward a consumed transaction's verifier. Legacy adapters fail closed on Some.
    ///
    /// # Errors
    /// Returns [`FederatedOAuthError::ExchangeCode`] if PKCE is unsupported or exchange fails.
    async fn exchange_code_with_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        if code_verifier.is_some() {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        self.exchange_code(code, redirect_uri).await
    }

    /// Fetch the vendor user profile with an access token.
    ///
    /// # Errors
    ///
    /// Returns [`FederatedOAuthError::UserProfile`] if the profile cannot be loaded.
    async fn user_profile(
        &self,
        access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError>;

    /// Build a v2 authentication URL with IAM's authentication context.
    ///
    /// Legacy adapters fail closed without calling any v1 operation.
    ///
    /// # Errors
    /// Returns [`FederatedOAuthError::Authorize`] unless v2 is implemented.
    async fn authorize_authentication(
        &self,
        _request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        Err(FederatedOAuthError::Authorize)
    }

    /// Complete v2 authentication and return tokens with the normalized identity.
    ///
    /// Legacy adapters fail closed without composing v1 token and profile calls.
    ///
    /// # Errors
    /// Returns [`FederatedOAuthError::ExchangeCode`] unless v2 is implemented.
    async fn complete_authorization(
        &self,
        _request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        Err(FederatedOAuthError::ExchangeCode)
    }
}
