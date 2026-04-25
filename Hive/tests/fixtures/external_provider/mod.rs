pub mod resources;
pub mod service;

pub struct ExternalProviderFixtures;

impl ExternalProviderFixtures {
    pub async fn service() -> service::ExternalProviderMockService {
        service::ExternalProviderMockService::new().await
    }
}
