use uuid::Uuid;

use crate::{
    entity::{ExampleEntity, EntityStatus},
    error::DomainError,
    port::{AuditLogRepository, ExampleEntityRepository},
};

/// Domain service for managing ExampleEntity business logic
pub struct ExampleEntityService<R, A>
where
    R: ExampleEntityRepository,
    A: AuditLogRepository,
{
    repository: R,
    audit_repository: A,
}

impl<R, A> ExampleEntityService<R, A>
where
    R: ExampleEntityRepository,
    A: AuditLogRepository,
{
    /// Create a new service instance
    pub fn new(repository: R, audit_repository: A) -> Self {
        Self {
            repository,
            audit_repository,
        }
    }

    /// Create a new entity with business rules
    pub async fn create_entity(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<ExampleEntity, DomainError> {
        // Business rule: Check if entity with same name already exists
        if self.repository.exists_by_name(&name).await? {
            return Err(DomainError::resource_already_exists(
                "ExampleEntity",
                &format!("name={}", name),
            ));
        }

        // Create the entity
        let entity = ExampleEntity::new(name, description)?;

        // Save to repository
        let saved_entity = self.repository.save(&entity).await?;

        // Log audit event
        self.audit_repository
            .log_event(
                &saved_entity.id,
                "ExampleEntity",
                "CREATE",
                Some(&format!("Created entity: {}", saved_entity.name)),
            )
            .await?;

        Ok(saved_entity)
    }

    /// Update an entity with business rules
    pub async fn update_entity(
        &self,
        id: &Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<ExampleEntity, DomainError> {
        // Find the entity
        let mut entity = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExampleEntity", &id.to_string()))?;

        // Apply updates
        if let Some(new_name) = name {
            // Business rule: Check if another entity with same name exists
            if new_name != entity.name && self.repository.exists_by_name(&new_name).await? {
                return Err(DomainError::resource_already_exists(
                    "ExampleEntity",
                    &format!("name={}", new_name),
                ));
            }
            entity.update_name(new_name)?;
        }

        if let Some(new_description) = description {
            entity.update_description(Some(new_description));
        }

        // Save updated entity
        let updated_entity = self.repository.save(&entity).await?;

        // Log audit event
        self.audit_repository
            .log_event(
                &updated_entity.id,
                "ExampleEntity",
                "UPDATE",
                Some(&format!("Updated entity: {}", updated_entity.name)),
            )
            .await?;

        Ok(updated_entity)
    }

    /// Activate an entity with business rules
    pub async fn activate_entity(&self, id: &Uuid) -> Result<ExampleEntity, DomainError> {
        let mut entity = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExampleEntity", &id.to_string()))?;

        entity.activate()?;
        let updated_entity = self.repository.save(&entity).await?;

        // Log audit event
        self.audit_repository
            .log_event(
                &updated_entity.id,
                "ExampleEntity",
                "ACTIVATE",
                Some(&format!("Activated entity: {}", updated_entity.name)),
            )
            .await?;

        Ok(updated_entity)
    }

    /// Deactivate an entity
    pub async fn deactivate_entity(&self, id: &Uuid) -> Result<ExampleEntity, DomainError> {
        let mut entity = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExampleEntity", &id.to_string()))?;

        entity.deactivate();
        let updated_entity = self.repository.save(&entity).await?;

        // Log audit event
        self.audit_repository
            .log_event(
                &updated_entity.id,
                "ExampleEntity",
                "DEACTIVATE",
                Some(&format!("Deactivated entity: {}", updated_entity.name)),
            )
            .await?;

        Ok(updated_entity)
    }

    /// Archive an entity with business rules
    pub async fn archive_entity(&self, id: &Uuid) -> Result<ExampleEntity, DomainError> {
        let mut entity = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExampleEntity", &id.to_string()))?;

        entity.archive()?;
        let updated_entity = self.repository.save(&entity).await?;

        // Log audit event
        self.audit_repository
            .log_event(
                &updated_entity.id,
                "ExampleEntity",
                "ARCHIVE",
                Some(&format!("Archived entity: {}", updated_entity.name)),
            )
            .await?;

        Ok(updated_entity)
    }

    /// Delete an entity (hard delete)
    pub async fn delete_entity(&self, id: &Uuid) -> Result<(), DomainError> {
        // Check if entity exists
        let entity = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExampleEntity", &id.to_string()))?;

        // Business rule: Can only delete inactive or archived entities
        match entity.status {
            EntityStatus::Active => {
                return Err(DomainError::business_rule_violation(
                    "Cannot delete active entity. Deactivate first.",
                ));
            }
            _ => {}
        }

        // Delete the entity
        self.repository.delete_by_id(id).await?;

        // Log audit event
        self.audit_repository
            .log_event(
                id,
                "ExampleEntity",
                "DELETE",
                Some(&format!("Deleted entity: {}", entity.name)),
            )
            .await?;

        Ok(())
    }

    /// Get entity by ID
    pub async fn get_entity(&self, id: &Uuid) -> Result<ExampleEntity, DomainError> {
        self.repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExampleEntity", &id.to_string()))
    }

    /// List all entities
    pub async fn list_entities(&self) -> Result<Vec<ExampleEntity>, DomainError> {
        self.repository.find_all().await
    }

    /// List only active entities
    pub async fn list_active_entities(&self) -> Result<Vec<ExampleEntity>, DomainError> {
        self.repository.find_all_active().await
    }

    /// Search entities by name
    pub async fn search_entities_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<ExampleEntity>, DomainError> {
        self.repository.find_by_name(name).await
    }

    /// Get total count of entities
    pub async fn count_entities(&self) -> Result<i64, DomainError> {
        self.repository.count().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{AuditLogEntry, AuditLogRepository, ExampleEntityRepository};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    // Mock repository for testing
    #[derive(Default)]
    pub struct MockExampleEntityRepository {
        entities: Arc<Mutex<Vec<ExampleEntity>>>,
    }

    #[async_trait]
    impl ExampleEntityRepository for MockExampleEntityRepository {
        async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExampleEntity>, DomainError> {
            let entities = self.entities.lock().unwrap();
            Ok(entities.iter().find(|e| e.id == *id).cloned())
        }

        async fn find_by_name(&self, name: &str) -> Result<Vec<ExampleEntity>, DomainError> {
            let entities = self.entities.lock().unwrap();
            Ok(entities
                .iter()
                .filter(|e| e.name.contains(name))
                .cloned()
                .collect())
        }

        async fn find_all(&self) -> Result<Vec<ExampleEntity>, DomainError> {
            let entities = self.entities.lock().unwrap();
            Ok(entities.clone())
        }

        async fn find_all_active(&self) -> Result<Vec<ExampleEntity>, DomainError> {
            let entities = self.entities.lock().unwrap();
            Ok(entities
                .iter()
                .filter(|e| e.is_active())
                .cloned()
                .collect())
        }

        async fn save(&self, entity: &ExampleEntity) -> Result<ExampleEntity, DomainError> {
            let mut entities = self.entities.lock().unwrap();
            if let Some(pos) = entities.iter().position(|e| e.id == entity.id) {
                entities[pos] = entity.clone();
            } else {
                entities.push(entity.clone());
            }
            Ok(entity.clone())
        }

        async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
            let mut entities = self.entities.lock().unwrap();
            entities.retain(|e| e.id != *id);
            Ok(())
        }

        async fn exists_by_name(&self, name: &str) -> Result<bool, DomainError> {
            let entities = self.entities.lock().unwrap();
            Ok(entities.iter().any(|e| e.name == name))
        }

        async fn count(&self) -> Result<i64, DomainError> {
            let entities = self.entities.lock().unwrap();
            Ok(entities.len() as i64)
        }
    }

    // Mock audit repository for testing
    #[derive(Default)]
    pub struct MockAuditLogRepository;

    #[async_trait]
    impl AuditLogRepository for MockAuditLogRepository {
        async fn log_event(
            &self,
            _entity_id: &Uuid,
            _entity_type: &str,
            _action: &str,
            _details: Option<&str>,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_audit_history(
            &self,
            _entity_id: &Uuid,
        ) -> Result<Vec<AuditLogEntry>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_create_entity() {
        let repo = MockExampleEntityRepository::default();
        let audit_repo = MockAuditLogRepository;
        let service = ExampleEntityService::new(repo, audit_repo);

        let result = service
            .create_entity("Test Entity".to_string(), Some("Description".to_string()))
            .await;

        assert!(result.is_ok());
        let entity = result.unwrap();
        assert_eq!(entity.name, "Test Entity");
        assert_eq!(entity.description, Some("Description".to_string()));
    }

    #[tokio::test]
    async fn test_create_duplicate_entity() {
        let repo = MockExampleEntityRepository::default();
        let audit_repo = MockAuditLogRepository;
        let service = ExampleEntityService::new(repo, audit_repo);

        // Create first entity
        let _first = service
            .create_entity("Test Entity".to_string(), None)
            .await
            .unwrap();

        // Try to create duplicate
        let result = service
            .create_entity("Test Entity".to_string(), None)
            .await;

        assert!(result.is_err());
        match result {
            Err(DomainError::ResourceAlreadyExists { .. }) => {}
            _ => panic!("Expected ResourceAlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_activate_entity() {
        let repo = MockExampleEntityRepository::default();
        let audit_repo = MockAuditLogRepository;
        let service = ExampleEntityService::new(repo, audit_repo);

        let entity = service
            .create_entity("Test Entity".to_string(), None)
            .await
            .unwrap();

        let result = service.activate_entity(&entity.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_active());
    }
} 