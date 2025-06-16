pub mod resources;
pub mod service;

/// Main GitLab fixtures namespace
pub struct GitLabFixtures;

impl GitLabFixtures {
    /// Create a new GitLab service instance
    pub async fn service() -> service::GitLabService {
        service::GitLabService::new().await
    }
}
