//! Auth provider factory for creating provider-specific authentication services

use crate::auth::{AuthError as OAuthError, OAuthService};
use iam_domain::entity::provider::Provider;
use std::sync::Arc;
use thiserror::Error;

/// Factory error type
#[derive(Debug, Error)]
pub enum FactoryError {
    /// Unsupported provider
    #[error("Unsupported provider: {0}")]
    #[allow(dead_code)]
    UnsupportedProvider(String),
}

/// Factory for creating authentication services based on provider type
pub struct OAuthProviderFactory<GH, GL>
where
    GH: OAuthService + 'static,
    GL: OAuthService + 'static,
{
    github_auth: Arc<GH>,
    gitlab_auth: Arc<GL>,
}

impl<GH, GL> OAuthProviderFactory<GH, GL>
where
    GH: OAuthService + 'static,
    GL: OAuthService + 'static,
{
    /// Create a new `AuthProviderFactory`
    pub const fn new(github_auth: Arc<GH>, gitlab_auth: Arc<GL>) -> Self {
        Self {
            github_auth,
            gitlab_auth,
        }
    }

    /// Get the authentication service for a specific provider
    /// Returns the concrete type wrapped in Arc with unified error type
    #[must_use]
    pub fn get_oauth_service(
        &self,
        provider: Provider,
    ) -> Arc<dyn OAuthService<Error = OAuthError>> {
        match provider {
            Provider::GitHub => Arc::new(OAuthServiceWrapper::new(self.github_auth.clone())),
            Provider::GitLab => Arc::new(OAuthServiceWrapper::new(self.gitlab_auth.clone())),
        }
    }
}

/// Wrapper to unify different auth service error types
struct OAuthServiceWrapper<AS: OAuthService> {
    inner: Arc<AS>,
}

impl<AS: OAuthService> OAuthServiceWrapper<AS> {
    const fn new(inner: Arc<AS>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<AS> OAuthService for OAuthServiceWrapper<AS>
where
    AS: OAuthService,
    AS::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = OAuthError;

    fn provider(&self) -> Provider {
        self.inner.provider()
    }

    fn generate_authorize_url(&self) -> String {
        self.inner.generate_authorize_url()
    }

    fn generate_relink_authorize_url(&self) -> String {
        self.inner.generate_relink_authorize_url()
    }

    async fn exchange_code(
        &self,
        code: String,
        redirect_uri: String,
    ) -> Result<
        (
            iam_domain::entity::provider::ProviderTokens,
            iam_domain::entity::provider::ProviderUserProfile,
        ),
        Self::Error,
    > {
        self.inner
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| OAuthError::AuthenticationError(e.to_string()))
    }
}
