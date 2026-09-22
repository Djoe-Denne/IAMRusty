//! Object-safe federated OAuth port (no vendor HTTP client).

use async_trait::async_trait;

use crate::dto::{AuthorizeResponse, ProviderTokens, ProviderUserProfile};
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

    /// Fetch the vendor user profile with an access token.
    ///
    /// # Errors
    ///
    /// Returns [`FederatedOAuthError::UserProfile`] if the profile cannot be loaded.
    async fn user_profile(
        &self,
        access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError>;
}
