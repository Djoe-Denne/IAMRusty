use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Example entity representing a core business object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExampleEntity {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: EntityStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status enumeration for the example entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityStatus {
    Active,
    Inactive,
    Pending,
    Archived,
}

impl ExampleEntity {
    /// Create a new example entity
    pub fn new(name: String, description: Option<String>) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            description,
            status: EntityStatus::Pending,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update the entity name
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        Self::validate_name(&new_name)?;
        self.name = new_name;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the entity description
    pub fn update_description(&mut self, new_description: Option<String>) {
        self.description = new_description;
        self.updated_at = Utc::now();
    }

    /// Activate the entity
    pub fn activate(&mut self) -> Result<(), DomainError> {
        match self.status {
            EntityStatus::Archived => {
                Err(DomainError::business_rule_violation(
                    "Cannot activate archived entity",
                ))
            }
            _ => {
                self.status = EntityStatus::Active;
                self.updated_at = Utc::now();
                Ok(())
            }
        }
    }

    /// Deactivate the entity
    pub fn deactivate(&mut self) {
        self.status = EntityStatus::Inactive;
        self.updated_at = Utc::now();
    }

    /// Archive the entity
    pub fn archive(&mut self) -> Result<(), DomainError> {
        match self.status {
            EntityStatus::Active => {
                Err(DomainError::business_rule_violation(
                    "Must deactivate entity before archiving",
                ))
            }
            _ => {
                self.status = EntityStatus::Archived;
                self.updated_at = Utc::now();
                Ok(())
            }
        }
    }

    /// Check if the entity is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, EntityStatus::Active)
    }

    /// Check if the entity is archived
    pub fn is_archived(&self) -> bool {
        matches!(self.status, EntityStatus::Archived)
    }

    /// Validate entity name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input("Name cannot be empty"));
        }

        if name.len() > 255 {
            return Err(DomainError::invalid_input(
                "Name cannot be longer than 255 characters",
            ));
        }

        Ok(())
    }
}

impl Default for EntityStatus {
    fn default() -> Self {
        EntityStatus::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_new_entity() {
        let entity = ExampleEntity::new(
            "Test Entity".to_string(),
            Some("Test Description".to_string()),
        );

        assert!(entity.is_ok());
        let entity = entity.unwrap();
        assert_eq!(entity.name, "Test Entity");
        assert_eq!(entity.description, Some("Test Description".to_string()));
        assert_eq!(entity.status, EntityStatus::Pending);
    }

    #[test]
    fn test_validate_name_empty() {
        let result = ExampleEntity::new("".to_string(), None);
        assert!(result.is_err());
        match result {
            Err(DomainError::InvalidInput { message }) => {
                assert_eq!(message, "Name cannot be empty");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_activate_entity() {
        let mut entity = ExampleEntity::new("Test".to_string(), None).unwrap();
        let result = entity.activate();
        
        assert!(result.is_ok());
        assert!(entity.is_active());
    }

    #[test]
    fn test_cannot_activate_archived_entity() {
        let mut entity = ExampleEntity::new("Test".to_string(), None).unwrap();
        entity.status = EntityStatus::Archived;
        
        let result = entity.activate();
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_workflow() {
        let mut entity = ExampleEntity::new("Test".to_string(), None).unwrap();
        
        // Cannot archive active entity
        entity.activate().unwrap();
        assert!(entity.archive().is_err());
        
        // Can archive after deactivating
        entity.deactivate();
        assert!(entity.archive().is_ok());
        assert!(entity.is_archived());
    }
} 