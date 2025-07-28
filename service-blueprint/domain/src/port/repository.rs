use async_trait::async_trait;
use uuid::Uuid;

use crate::{entity::ExampleEntity, error::DomainError};

/// Repository port for ExampleEntity
#[async_trait]
pub trait ExampleEntityRepository: Send + Sync {
    /// Find an entity by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExampleEntity>, DomainError>;

    /// Find entities by name (partial match)
    async fn find_by_name(&self, name: &str) -> Result<Vec<ExampleEntity>, DomainError>;

    /// Find all entities
    async fn find_all(&self) -> Result<Vec<ExampleEntity>, DomainError>;

    /// Find all active entities
    async fn find_all_active(&self) -> Result<Vec<ExampleEntity>, DomainError>;

    /// Save an entity (create or update)
    async fn save(&self, entity: &ExampleEntity) -> Result<ExampleEntity, DomainError>;

    /// Delete an entity by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Check if an entity exists by name
    async fn exists_by_name(&self, name: &str) -> Result<bool, DomainError>;

    /// Count total entities
    async fn count(&self) -> Result<i64, DomainError>;
}

/// Example of another repository port
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// Log an audit event
    async fn log_event(
        &self,
        entity_id: &Uuid,
        entity_type: &str,
        action: &str,
        details: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Get audit history for an entity
    async fn get_audit_history(
        &self,
        entity_id: &Uuid,
    ) -> Result<Vec<AuditLogEntry>, DomainError>;
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub action: String,
    pub details: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
} 