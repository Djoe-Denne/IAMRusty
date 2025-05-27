//! Auth provider factory for creating provider-specific authentication services

use crate::auth::{AuthService, AuthError};
use domain::entity::provider::Provider;
use std::sync::Arc;
use thiserror::Error;

/// Factory error type
#[derive(Debug, Error)]
pub enum FactoryError {
    /// Unsupported provider
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
}

/// Factory for creating authentication services based on provider type
pub struct AuthProviderFactory<GH, GL>
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
{
    github_auth: Arc<GH>,
    gitlab_auth: Arc<GL>,
}

impl<GH, GL> AuthProviderFactory<GH, GL>
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
{
    /// Create a new AuthProviderFactory
    pub fn new(github_auth: Arc<GH>, gitlab_auth: Arc<GL>) -> Self {
        Self {
            github_auth,
            gitlab_auth,
        }
    }

    /// Get the authentication service for a specific provider
    /// Returns the concrete type wrapped in Arc with unified error type
    pub fn get_auth_service(&self, provider: Provider) -> Arc<dyn AuthService<Error = AuthError>> {
        match provider {
            Provider::GitHub => Arc::new(AuthServiceWrapper::new(self.github_auth.clone())),
            Provider::GitLab => Arc::new(AuthServiceWrapper::new(self.gitlab_auth.clone())),
        }
    }
}

/// Wrapper to unify different auth service error types
struct AuthServiceWrapper<AS: AuthService> {
    inner: Arc<AS>,
}

impl<AS: AuthService> AuthServiceWrapper<AS> {
    fn new(inner: Arc<AS>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<AS> AuthService for AuthServiceWrapper<AS>
where
    AS: AuthService,
    AS::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = AuthError;

    fn provider(&self) -> Provider {
        self.inner.provider()
    }

    fn generate_authorize_url(&self) -> String {
        self.inner.generate_authorize_url()
    }

    async fn exchange_code(
        &self,
        code: String,
        redirect_uri: String,
    ) -> Result<(domain::entity::provider::ProviderTokens, domain::entity::provider::ProviderUserProfile), Self::Error> {
        self.inner
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| AuthError::AuthenticationError(e.to_string()))
    }
} 