//! Wiremock remote signer fixture (ADR-0309).

pub mod resources;
pub mod service;

pub struct RemoteSignerFixtures;

impl RemoteSignerFixtures {
    pub async fn service() -> service::RemoteSignerMockService {
        service::RemoteSignerMockService::new().await
    }
}
