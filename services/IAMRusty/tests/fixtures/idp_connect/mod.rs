pub mod resources;
pub mod service;

/// Namespace for the IdP Connect WireMock fixture.
pub struct IdpConnectFixtures;

impl IdpConnectFixtures {
    /// Create a new IdP Connect mock service (binds shared :3000).
    pub async fn service() -> service::IdpConnectMockService {
        service::IdpConnectMockService::new().await
    }
}
