use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use rustycog_core::error::DomainError;

/// Organization entity representing a business organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub owner_user_id: Uuid,
    pub settings: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Create a new organization
    pub fn new(
        name: String,
        slug: String,
        description: Option<String>,
        owner_user_id: Uuid,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        Self::validate_slug(&slug)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            slug,
            description,
            avatar_url: None,
            owner_user_id,
            settings: serde_json::json!({
                "visibility": "Public",
            }),
            created_at: now,
            updated_at: now,
        })
    }

    /// Update organization name
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        Self::validate_name(&new_name)?;
        self.name = new_name;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update organization description
    pub fn update_description(&mut self, new_description: Option<String>) {
        if let Some(ref desc) = new_description {
            if desc.len() > 1000 {
                return; // Silently ignore invalid description
            }
        }
        self.description = new_description;
        self.updated_at = Utc::now();
    }

    /// Update organization avatar URL
    pub fn update_avatar_url(&mut self, new_avatar_url: Option<String>) {
        self.avatar_url = new_avatar_url;
        self.updated_at = Utc::now();
    }

    /// Update organization settings
    pub fn update_settings(&mut self, new_settings: Value) {
        self.settings = new_settings;
        self.updated_at = Utc::now();
    }

    /// Check if user is the owner of this organization
    pub fn is_owned_by(&self, user_id: &Uuid) -> bool {
        self.owner_user_id == *user_id
    }

    /// Validate organization name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input(
                "Organization name cannot be empty",
            ));
        }

        if name.len() > 255 {
            return Err(DomainError::invalid_input(
                "Organization name cannot be longer than 255 characters",
            ));
        }

        Ok(())
    }

    /// Validate organization slug
    fn validate_slug(slug: &str) -> Result<(), DomainError> {
        if slug.trim().is_empty() {
            return Err(DomainError::invalid_input(
                "Organization slug cannot be empty",
            ));
        }

        if slug.len() > 100 {
            return Err(DomainError::invalid_input(
                "Organization slug cannot be longer than 100 characters",
            ));
        }

        // Check slug format (alphanumeric and hyphens only)
        if !slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(DomainError::invalid_input(
                "Organization slug can only contain alphanumeric characters and hyphens",
            ));
        }

        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(DomainError::invalid_input(
                "Organization slug cannot start or end with a hyphen",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_organization() {
        let owner_id = Uuid::new_v4();
        let org = Organization::new(
            "Test Org".to_string(),
            "test-org".to_string(),
            Some("Test Description".to_string()),
            owner_id,
        );

        assert!(org.is_ok());
        let org = org.unwrap();
        assert_eq!(org.name, "Test Org");
        assert_eq!(org.slug, "test-org");
        assert_eq!(org.description, Some("Test Description".to_string()));
        assert_eq!(org.owner_user_id, owner_id);
        assert!(org.is_owned_by(&owner_id));
    }

    #[test]
    fn test_validate_name_empty() {
        let owner_id = Uuid::new_v4();
        let result = Organization::new("".to_string(), "test".to_string(), None, owner_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_slug_invalid_format() {
        let owner_id = Uuid::new_v4();

        // Test invalid characters
        let result = Organization::new("Test".to_string(), "test@org".to_string(), None, owner_id);
        assert!(result.is_err());

        // Test starting with hyphen
        let result = Organization::new("Test".to_string(), "-test".to_string(), None, owner_id);
        assert!(result.is_err());

        // Test ending with hyphen
        let result = Organization::new("Test".to_string(), "test-".to_string(), None, owner_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_organization() {
        let owner_id = Uuid::new_v4();
        let mut org = Organization::new(
            "Test Org".to_string(),
            "test-org".to_string(),
            None,
            owner_id,
        )
        .unwrap();

        let original_updated_at = org.updated_at;

        // Update name
        let result = org.update_name("Updated Org".to_string());
        assert!(result.is_ok());
        assert_eq!(org.name, "Updated Org");
        assert!(org.updated_at > original_updated_at);

        // Update description
        org.update_description(Some("New description".to_string()));
        assert_eq!(org.description, Some("New description".to_string()));
    }
}
