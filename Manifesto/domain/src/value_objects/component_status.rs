use rustycog::core::error::DomainError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Pending,
    Configured,
    Active,
    Disabled,
}

impl ComponentStatus {
    /// Check if this status can transition to the target status
    #[must_use]
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            // Pending→Configured, Configured→Active, Active→Disabled
            (Self::Pending, Self::Configured)
            | (Self::Configured, Self::Active)
            | (Self::Active, Self::Disabled) => true,
            // Same status is always allowed (no-op)
            (current, target) if current == target => true,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Attempt to transition to the target status.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the transition from the current status to `target` is not allowed.
    pub fn transition_to(&self, target: Self) -> Result<Self, DomainError> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(DomainError::business_rule_violation(&format!(
                "Cannot transition component from {self:?} to {target:?}"
            )))
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Configured => "configured",
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}

impl FromStr for ComponentStatus {
    type Err = DomainError;

    /// Parse a component status from a string.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `s` is not a recognized component status.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "configured" => Ok(Self::Configured),
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid component status: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for ComponentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(ComponentStatus::Pending.can_transition_to(&ComponentStatus::Configured));
        assert!(ComponentStatus::Configured.can_transition_to(&ComponentStatus::Active));
        assert!(ComponentStatus::Active.can_transition_to(&ComponentStatus::Disabled));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!ComponentStatus::Pending.can_transition_to(&ComponentStatus::Active));
        assert!(!ComponentStatus::Configured.can_transition_to(&ComponentStatus::Disabled));
        assert!(!ComponentStatus::Disabled.can_transition_to(&ComponentStatus::Active));
    }
}
