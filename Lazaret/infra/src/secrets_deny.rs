//! Fail-closed secret resolver when Vault is not configured.

use async_trait::async_trait;
use lazaret_domain::{SecretError, SecretResolver};

/// Always fails. Used when `[vault]` is empty.
pub struct DeniedSecretResolver;

#[async_trait]
impl SecretResolver for DeniedSecretResolver {
    async fn resolve(&self, _reference: &str) -> Result<Vec<u8>, SecretError> {
        Err(SecretError::ResolveFailed)
    }
}
