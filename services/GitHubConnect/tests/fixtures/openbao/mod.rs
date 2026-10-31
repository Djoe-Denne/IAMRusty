pub mod resources;
pub mod service;

pub struct OpenBaoFixtures;
impl OpenBaoFixtures {
    pub async fn service() -> service::OpenBaoService {
        service::OpenBaoService::new().await
    }
}
