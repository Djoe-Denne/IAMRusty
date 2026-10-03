//! Namespace for the Vault HTTP KV fake.

pub mod resources;
pub mod service;

/// Factory for [`service::VaultMockService`].
pub struct VaultFixtures;

impl VaultFixtures {
    /// Isolated wiremock of Vault KV v2.
    pub async fn service() -> service::VaultMockService {
        service::VaultMockService::new().await
    }
}
