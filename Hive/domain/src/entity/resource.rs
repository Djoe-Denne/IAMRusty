use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Resource types available in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Org,
    Member,
    Roles,
    Issues,
    External,
    Invitations,
}

impl ResourceType {
    /// Get all available resource types
    pub fn all() -> Vec<ResourceType> {
        vec![
            ResourceType::Org,
            ResourceType::Member,
            ResourceType::Roles,
            ResourceType::Issues,
            ResourceType::External,
            ResourceType::Invitations,
        ]
    }
    
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Org => "org",
            ResourceType::Member => "member",
            ResourceType::Roles => "roles",
            ResourceType::Issues => "issues",
            ResourceType::External => "external",
            ResourceType::Invitations => "invitations",
        }
    }
    
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "org" => Ok(ResourceType::Org),
            "member" => Ok(ResourceType::Member),
            "roles" => Ok(ResourceType::Roles),
            "issues" => Ok(ResourceType::Issues),
            "external" => Ok(ResourceType::External),
            "invitations" => Ok(ResourceType::Invitations),
            _ => Err(DomainError::invalid_input(&format!("Invalid resource type: {}", s))),
        }
    }
}

/// Resource entity representing a specific resource in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub resource_type: ResourceType,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Resource {
    /// Create a new resource
    pub fn new(
        resource_type: ResourceType,
        name: String,
        description: Option<String>,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        
        Ok(Self {
            id: Uuid::new_v4(),
            resource_type,
            name,
            description,
            created_at: Utc::now(),
        })
    }
    
    /// Create system resources
    pub fn create_system_resources() -> Result<Vec<Resource>, DomainError> {
        let resources = vec![
            (ResourceType::Org, "Organization", "Organization management resources"),
            (ResourceType::Member, "Members", "Organization member management"),
            (ResourceType::Roles, "Roles", "Role and permission management"),
            (ResourceType::Issues, "Issues", "Issue tracking and management"),
            (ResourceType::External, "External", "External integrations and providers"),
            (ResourceType::Invitations, "Invitations", "Organization invitations"),
        ];
        
        resources
            .into_iter()
            .map(|(resource_type, name, description)| {
                Resource::new(
                    resource_type,
                    name.to_string(),
                    Some(description.to_string()),
                )
            })
            .collect()
    }
    
    /// Update resource name
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        Self::validate_name(&new_name)?;
        self.name = new_name;
        Ok(())
    }
    
    /// Update resource description
    pub fn update_description(&mut self, new_description: Option<String>) {
        self.description = new_description;
    }
    
    /// Validate resource name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input("Resource name cannot be empty"));
        }
        
        if name.len() > 100 {
            return Err(DomainError::invalid_input(
                "Resource name cannot be longer than 100 characters",
            ));
        }
        
        Ok(())
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
} 