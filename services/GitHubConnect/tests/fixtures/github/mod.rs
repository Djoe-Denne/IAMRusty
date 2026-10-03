pub mod resources;
pub mod service;

/// Main GitHub fixtures namespace
pub struct GitHubFixtures;

impl GitHubFixtures {
    /// Create a new GitHub service instance
    pub async fn service() -> service::GitHubService {
        service::GitHubService::new().await
    }
}
