use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::DomainError;

/// External provider entity representing third-party provider configurations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalProvider {
    pub id: Uuid,
    pub provider_type: ProviderType,
    pub name: String,
    pub config_schema: Option<Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Supported external provider types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderType {
    GitHub,
    GitLab,
    Confluence,
}

impl ExternalProvider {
    /// Create a new external provider
    pub fn new(
        provider_type: ProviderType,
        name: String,
        config_schema: Option<Value>,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;

        Ok(Self {
            id: Uuid::new_v4(),
            provider_type,
            name,
            config_schema,
            is_active: true,
            created_at: Utc::now(),
        })
    }

    /// Create a GitHub provider
    pub fn new_github() -> Self {
        let config_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "org_name": {
                    "type": "string",
                    "description": "GitHub organization name"
                },
                "access_token": {
                    "type": "string",
                    "description": "GitHub access token"
                },
                "base_url": {
                    "type": "string",
                    "description": "GitHub API base URL (for GitHub Enterprise)",
                    "default": "https://api.github.com"
                }
            },
            "required": ["org_name", "access_token"]
        });

        Self {
            id: Uuid::new_v4(),
            provider_type: ProviderType::GitHub,
            name: "GitHub".to_string(),
            config_schema: Some(config_schema),
            is_active: true,
            created_at: Utc::now(),
        }
    }

    /// Create a GitLab provider
    pub fn new_gitlab() -> Self {
        let config_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "group_id": {
                    "type": "string",
                    "description": "GitLab group ID"
                },
                "access_token": {
                    "type": "string",
                    "description": "GitLab access token"
                },
                "base_url": {
                    "type": "string",
                    "description": "GitLab instance URL",
                    "default": "https://gitlab.com"
                }
            },
            "required": ["group_id", "access_token"]
        });

        Self {
            id: Uuid::new_v4(),
            provider_type: ProviderType::GitLab,
            name: "GitLab".to_string(),
            config_schema: Some(config_schema),
            is_active: true,
            created_at: Utc::now(),
        }
    }

    /// Create a Confluence provider
    pub fn new_confluence() -> Self {
        let config_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "space_key": {
                    "type": "string",
                    "description": "Confluence space key"
                },
                "api_token": {
                    "type": "string",
                    "description": "Confluence API token"
                },
                "username": {
                    "type": "string",
                    "description": "Confluence username"
                },
                "base_url": {
                    "type": "string",
                    "description": "Confluence instance URL"
                }
            },
            "required": ["space_key", "api_token", "username", "base_url"]
        });

        Self {
            id: Uuid::new_v4(),
            provider_type: ProviderType::Confluence,
            name: "Confluence".to_string(),
            config_schema: Some(config_schema),
            is_active: true,
            created_at: Utc::now(),
        }
    }

    /// Update provider name
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        Self::validate_name(&new_name)?;
        self.name = new_name;
        Ok(())
    }

    /// Update config schema
    pub fn update_config_schema(&mut self, new_schema: Option<Value>) {
        self.config_schema = new_schema;
    }

    /// Activate the provider
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Deactivate the provider
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Validate provider name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input("Provider name cannot be empty"));
        }

        if name.len() > 100 {
            return Err(DomainError::invalid_input(
                "Provider name cannot be longer than 100 characters",
            ));
        }

        Ok(())
    }
}

impl ProviderType {
    /// Get string representation of provider type
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::GitHub => "github",
            ProviderType::GitLab => "gitlab",
            ProviderType::Confluence => "confluence",
        }
    }

    /// Parse provider type from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "github" => Ok(ProviderType::GitHub),
            "gitlab" => Ok(ProviderType::GitLab),
            "confluence" => Ok(ProviderType::Confluence),
            _ => Err(DomainError::invalid_input(&format!(
                "Unknown provider type: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_external_provider() {
        let provider = ExternalProvider::new(
            ProviderType::GitHub,
            "Custom GitHub".to_string(),
            None,
        );

        assert!(provider.is_ok());
        let provider = provider.unwrap();
        assert_eq!(provider.name, "Custom GitHub");
        assert!(matches!(provider.provider_type, ProviderType::GitHub));
        assert!(provider.is_active);
    }

    #[test]
    fn test_create_github_provider() {
        let provider = ExternalProvider::new_github();

        assert_eq!(provider.name, "GitHub");
        assert!(matches!(provider.provider_type, ProviderType::GitHub));
        assert!(provider.config_schema.is_some());
        assert!(provider.is_active);
    }

    #[test]
    fn test_create_gitlab_provider() {
        let provider = ExternalProvider::new_gitlab();

        assert_eq!(provider.name, "GitLab");
        assert!(matches!(provider.provider_type, ProviderType::GitLab));
        assert!(provider.config_schema.is_some());
    }

    #[test]
    fn test_create_confluence_provider() {
        let provider = ExternalProvider::new_confluence();

        assert_eq!(provider.name, "Confluence");
        assert!(matches!(provider.provider_type, ProviderType::Confluence));
        assert!(provider.config_schema.is_some());
    }

    #[test]
    fn test_provider_type_conversion() {
        assert_eq!(ProviderType::GitHub.as_str(), "github");
        assert_eq!(ProviderType::GitLab.as_str(), "gitlab");
        assert_eq!(ProviderType::Confluence.as_str(), "confluence");

        assert!(matches!(
            ProviderType::from_str("github").unwrap(),
            ProviderType::GitHub
        ));
        assert!(matches!(
            ProviderType::from_str("GITLAB").unwrap(),
            ProviderType::GitLab
        ));
        assert!(ProviderType::from_str("invalid").is_err());
    }

    #[test]
    fn test_validate_name() {
        let result = ExternalProvider::new(
            ProviderType::GitHub,
            "".to_string(),
            None,
        );
        assert!(result.is_err());

        let result = ExternalProvider::new(
            ProviderType::GitHub,
            "a".repeat(101),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_activate_deactivate() {
        let mut provider = ExternalProvider::new_github();

        provider.deactivate();
        assert!(!provider.is_active);

        provider.activate();
        assert!(provider.is_active);
    }
} 