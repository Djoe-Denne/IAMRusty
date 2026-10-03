//! Error types for the federated OAuth trait (not HMAC).

use thiserror::Error;

/// Failure from a [`crate::FederatedOAuthClient`] implementation.
///
/// Variants carry no secret, authorization code, or access token. Connectors
/// must not put those values in logs either.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum FederatedOAuthError {
    /// `authorize` could not build an authorization URL.
    #[error("federated authorize failed")]
    Authorize,
    /// `exchange_code` could not obtain tokens.
    #[error("federated token exchange failed")]
    ExchangeCode,
    /// `user_profile` could not load a profile.
    #[error("federated user profile failed")]
    UserProfile,
}
