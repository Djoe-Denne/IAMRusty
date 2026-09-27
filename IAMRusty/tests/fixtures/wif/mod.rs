//! Wiremock WIF (AWS / GCP / Azure) fixture (ADR-0307).

pub mod resources;
pub mod service;

pub struct WifFixtures;

impl WifFixtures {
    pub async fn service() -> service::WifMockService {
        service::WifMockService::new().await
    }
}
