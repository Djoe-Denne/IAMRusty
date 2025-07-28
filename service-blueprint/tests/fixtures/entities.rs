use uuid::Uuid;

use {{SERVICE_NAME}}_domain::{ExampleEntity, EntityStatus};

/// Entity test fixtures
pub struct EntityFixtures;

impl EntityFixtures {
    /// Create a basic test entity
    pub fn basic_entity() -> ExampleEntity {
        ExampleEntity::new(
            "Basic Test Entity".to_string(),
            Some("This is a basic test entity".to_string()),
        ).unwrap()
    }
    
    /// Create an entity with minimal data
    pub fn minimal_entity() -> ExampleEntity {
        ExampleEntity::new(
            "Minimal Entity".to_string(),
            None,
        ).unwrap()
    }
    
    /// Create an entity with a very long name (for testing validation)
    pub fn entity_with_long_name() -> Result<ExampleEntity, {{SERVICE_NAME}}_domain::DomainError> {
        ExampleEntity::new(
            "A".repeat(250), // Just under the limit
            Some("Entity with a long name for testing".to_string()),
        )
    }
    
    /// Create an entity with a very long description
    pub fn entity_with_long_description() -> ExampleEntity {
        ExampleEntity::new(
            "Entity with Long Description".to_string(),
            Some("A".repeat(900)), // Long but valid description
        ).unwrap()
    }
    
    /// Create an active entity
    pub fn active_entity() -> ExampleEntity {
        let mut entity = Self::basic_entity();
        entity.activate().unwrap();
        entity
    }
    
    /// Create an inactive entity
    pub fn inactive_entity() -> ExampleEntity {
        let mut entity = Self::active_entity();
        entity.deactivate();
        entity
    }
    
    /// Create an archived entity
    pub fn archived_entity() -> ExampleEntity {
        let mut entity = Self::basic_entity();
        entity.deactivate(); // Must deactivate before archiving
        entity.archive().unwrap();
        entity
    }
    
    /// Create multiple entities with different statuses
    pub fn entities_with_different_statuses() -> Vec<ExampleEntity> {
        vec![
            Self::basic_entity(),     // Pending
            Self::active_entity(),    // Active
            Self::inactive_entity(),  // Inactive
            Self::archived_entity(),  // Archived
        ]
    }
    
    /// Create entities for pagination testing
    pub fn entities_for_pagination(count: usize) -> Vec<ExampleEntity> {
        (0..count)
            .map(|i| {
                ExampleEntity::new(
                    format!("Entity {}", i + 1),
                    Some(format!("Description for entity {}", i + 1)),
                ).unwrap()
            })
            .collect()
    }
    
    /// Create entities with specific names for search testing
    pub fn entities_for_search() -> Vec<ExampleEntity> {
        vec![
            ExampleEntity::new("Apple Product".to_string(), None).unwrap(),
            ExampleEntity::new("Banana Service".to_string(), None).unwrap(),
            ExampleEntity::new("Cherry Application".to_string(), None).unwrap(),
            ExampleEntity::new("Date Tool".to_string(), None).unwrap(),
            ExampleEntity::new("Elderberry System".to_string(), None).unwrap(),
        ]
    }
    
    /// Try to create an entity with invalid data (for negative testing)
    pub fn invalid_entity_empty_name() -> Result<ExampleEntity, {{SERVICE_NAME}}_domain::DomainError> {
        ExampleEntity::new("".to_string(), None)
    }
    
    /// Try to create an entity with invalid data (for negative testing)
    pub fn invalid_entity_too_long_name() -> Result<ExampleEntity, {{SERVICE_NAME}}_domain::DomainError> {
        ExampleEntity::new("A".repeat(300), None) // Over the limit
    }
}

/// Builder pattern for creating custom test entities
pub struct EntityBuilder {
    name: String,
    description: Option<String>,
    status: Option<EntityStatus>,
}

impl EntityBuilder {
    pub fn new() -> Self {
        Self {
            name: "Test Entity".to_string(),
            description: None,
            status: None,
        }
    }
    
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    
    pub fn with_status(mut self, status: EntityStatus) -> Self {
        self.status = Some(status);
        self
    }
    
    pub fn build(self) -> Result<ExampleEntity, {{SERVICE_NAME}}_domain::DomainError> {
        let mut entity = ExampleEntity::new(self.name, self.description)?;
        
        if let Some(status) = self.status {
            match status {
                EntityStatus::Active => { entity.activate()?; },
                EntityStatus::Inactive => {
                    entity.activate()?;
                    entity.deactivate();
                },
                EntityStatus::Archived => {
                    entity.deactivate();
                    entity.archive()?;
                },
                EntityStatus::Pending => {
                    // Already pending by default
                },
            }
        }
        
        Ok(entity)
    }
}

impl Default for EntityBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_entity_fixture() {
        let entity = EntityFixtures::basic_entity();
        assert_eq!(entity.name, "Basic Test Entity");
        assert!(entity.description.is_some());
        assert!(matches!(entity.status, EntityStatus::Pending));
    }

    #[test]
    fn test_active_entity_fixture() {
        let entity = EntityFixtures::active_entity();
        assert!(entity.is_active());
    }

    #[test]
    fn test_archived_entity_fixture() {
        let entity = EntityFixtures::archived_entity();
        assert!(entity.is_archived());
    }

    #[test]
    fn test_entities_with_different_statuses() {
        let entities = EntityFixtures::entities_with_different_statuses();
        assert_eq!(entities.len(), 4);
        
        assert!(matches!(entities[0].status, EntityStatus::Pending));
        assert!(matches!(entities[1].status, EntityStatus::Active));
        assert!(matches!(entities[2].status, EntityStatus::Inactive));
        assert!(matches!(entities[3].status, EntityStatus::Archived));
    }

    #[test]
    fn test_entity_builder() {
        let entity = EntityBuilder::new()
            .with_name("Custom Entity")
            .with_description("Custom description")
            .with_status(EntityStatus::Active)
            .build()
            .unwrap();
        
        assert_eq!(entity.name, "Custom Entity");
        assert_eq!(entity.description, Some("Custom description".to_string()));
        assert!(entity.is_active());
    }

    #[test]
    fn test_invalid_entity_fixtures() {
        assert!(EntityFixtures::invalid_entity_empty_name().is_err());
        assert!(EntityFixtures::invalid_entity_too_long_name().is_err());
    }
} 