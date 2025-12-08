use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

/// Resource type indicating whether a resource is internal or a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    /// Internal resources like "project" and "member"
    Internal,
    /// Dynamic component resources added to projects
    Component,
}

impl ResourceType {
    /// Convert from string representation
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "internal" => Ok(ResourceType::Internal),
            "component" => Ok(ResourceType::Component),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid resource type: {}",
                s
            ))),
        }
    }

    /// Convert to string representation
    pub fn to_str(&self) -> &'static str {
        match self {
            ResourceType::Internal => "internal",
            ResourceType::Component => "component",
        }
    }

    /// Check if this is an internal resource
    pub fn is_internal(&self) -> bool {
        matches!(self, ResourceType::Internal)
    }

    /// Check if this is a component resource
    pub fn is_component(&self) -> bool {
        matches!(self, ResourceType::Component)
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(
            ResourceType::from_str("internal").unwrap(),
            ResourceType::Internal
        );
        assert_eq!(
            ResourceType::from_str("COMPONENT").unwrap(),
            ResourceType::Component
        );
        assert!(ResourceType::from_str("invalid").is_err());
    }

    #[test]
    fn test_is_checks() {
        assert!(ResourceType::Internal.is_internal());
        assert!(!ResourceType::Internal.is_component());
        assert!(ResourceType::Component.is_component());
        assert!(!ResourceType::Component.is_internal());
    }
}

