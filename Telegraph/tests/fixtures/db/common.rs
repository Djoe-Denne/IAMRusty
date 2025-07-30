use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Common trait for all entity fixture builders
#[async_trait]
pub trait FixtureBuilder<T> {
    /// Commit the fixture to the database
    async fn commit(self, db: DatabaseConnection) -> anyhow::Result<T>;

    /// Check if the entity exists in the database with expected values
    async fn check(&self, db: &DatabaseConnection, entity: &T) -> anyhow::Result<bool>;
}

/// Trait for providing factory methods for common test scenarios
pub trait FixtureFactory<T> {
    /// Create a builder with common defaults
    fn default() -> T;
}
