//! Namespace for the Manifesto binding grant snapshot fake.

pub mod resources;
pub mod service;

/// Factory for [`service::BindingSnapshotMockService`].
pub struct BindingSnapshotFixtures;

impl BindingSnapshotFixtures {
    /// Isolated wiremock service.
    pub async fn service() -> service::BindingSnapshotMockService {
        service::BindingSnapshotMockService::new().await
    }
}
