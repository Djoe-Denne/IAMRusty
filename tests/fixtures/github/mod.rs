pub mod service;
pub mod resources;

pub use service::*;
pub use resources::*;

/// Main GitHub fixtures namespace
pub struct GitHubFixtures;

impl GitHubFixtures {
    /// Create a new GitHub service instance
    pub async fn service() -> service::GitHubService {
        service::GitHubService::new().await
    }
}

/// Re-export resources for easy access
pub use resources::*; 