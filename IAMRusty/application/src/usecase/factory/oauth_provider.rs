//! Auth provider factory for creating provider-specific authentication services

use iam_domain::entity::provider::Provider;
use iam_domain::port::service::FederatedOAuthClient;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Factory error type
#[derive(Debug, Error)]
pub enum FactoryError {
    /// Unsupported or unregistered provider
    #[error("Unsupported provider: {0}")]
    #[allow(dead_code)]
    UnsupportedProvider(String),
}

/// Factory wrapping one federated client per provider slug.
pub struct OAuthProviderFactory {
    clients: HashMap<Provider, Arc<dyn FederatedOAuthClient>>,
}

impl OAuthProviderFactory {
    /// Create a factory from a per-slug client map.
    #[must_use]
    pub fn new(clients: HashMap<Provider, Arc<dyn FederatedOAuthClient>>) -> Self {
        Self { clients }
    }

    /// Get the federated client for a provider slug.
    ///
    /// # Errors
    ///
    /// Returns [`FactoryError::UnsupportedProvider`] when the slug has no registered client.
    pub fn get(&self, provider: Provider) -> Result<Arc<dyn FederatedOAuthClient>, FactoryError> {
        self.clients
            .get(&provider)
            .cloned()
            .ok_or_else(|| FactoryError::UnsupportedProvider(provider.as_str().to_string()))
    }
}
