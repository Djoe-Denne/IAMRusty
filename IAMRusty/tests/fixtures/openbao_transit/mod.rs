//! OpenBao Transit wiremock fixture (ADR-0304).

pub mod resources;
pub mod service;

pub struct OpenBaoTransitFixtures;

impl OpenBaoTransitFixtures {
    pub async fn service() -> service::OpenBaoTransitMockService {
        service::OpenBaoTransitMockService::new().await
    }
}
