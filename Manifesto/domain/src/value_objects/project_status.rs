use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Draft,
    Active,
    Archived,
    Suspended,
}

impl ProjectStatus {
    /// Check if this status can transition to the target status
    pub fn can_transition_to(&self, target: &ProjectStatus) -> bool {
        match (self, target) {
            // From Draft
            (Self::Draft, Self::Active) => true,
            // From Active
            (Self::Active, Self::Archived | Self::Suspended) => true,
            // Same status is always allowed (no-op)
            (current, target) if current == target => true,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Attempt to transition to the target status
    pub fn transition_to(&self, target: ProjectStatus) -> Result<ProjectStatus, DomainError> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(DomainError::business_rule_violation(
                &format!("Cannot transition project from {:?} to {:?}", self, target),
            ))
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Suspended => "suspended",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "suspended" => Ok(Self::Suspended),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid project status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(ProjectStatus::Draft.can_transition_to(&ProjectStatus::Active));
        assert!(ProjectStatus::Active.can_transition_to(&ProjectStatus::Archived));
        assert!(ProjectStatus::Active.can_transition_to(&ProjectStatus::Suspended));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!ProjectStatus::Draft.can_transition_to(&ProjectStatus::Archived));
        assert!(!ProjectStatus::Archived.can_transition_to(&ProjectStatus::Active));
        assert!(!ProjectStatus::Suspended.can_transition_to(&ProjectStatus::Active));
    }

    #[test]
    fn test_same_status_allowed() {
        assert!(ProjectStatus::Draft.can_transition_to(&ProjectStatus::Draft));
        assert!(ProjectStatus::Active.can_transition_to(&ProjectStatus::Active));
    }
}

