use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

/// Permission level representing hierarchical access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Read,
    Write,
    Admin,
    Owner,
}

impl PermissionLevel {
    /// Convert from string representation
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "read" => Ok(PermissionLevel::Read),
            "write" => Ok(PermissionLevel::Write),
            "admin" => Ok(PermissionLevel::Admin),
            "owner" => Ok(PermissionLevel::Owner),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid permission level: {}",
                s
            ))),
        }
    }

    /// Convert to string representation
    pub fn to_str(&self) -> &'static str {
        match self {
            PermissionLevel::Read => "read",
            PermissionLevel::Write => "write",
            PermissionLevel::Admin => "admin",
            PermissionLevel::Owner => "owner",
        }
    }

    /// Check if this permission level has at least the required permission
    pub fn has_permission(&self, required: &PermissionLevel) -> bool {
        self >= required
    }

    /// Check if this permission can manage another permission level
    pub fn can_manage(&self, other: &PermissionLevel) -> bool {
        self > other
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy() {
        assert!(PermissionLevel::Owner > PermissionLevel::Admin);
        assert!(PermissionLevel::Admin > PermissionLevel::Write);
        assert!(PermissionLevel::Write > PermissionLevel::Read);
    }

    #[test]
    fn test_has_permission() {
        assert!(PermissionLevel::Owner.has_permission(&PermissionLevel::Read));
        assert!(PermissionLevel::Admin.has_permission(&PermissionLevel::Write));
        assert!(PermissionLevel::Write.has_permission(&PermissionLevel::Write));
        assert!(!PermissionLevel::Read.has_permission(&PermissionLevel::Write));
    }

    #[test]
    fn test_can_manage() {
        assert!(PermissionLevel::Owner.can_manage(&PermissionLevel::Admin));
        assert!(PermissionLevel::Admin.can_manage(&PermissionLevel::Write));
        assert!(!PermissionLevel::Write.can_manage(&PermissionLevel::Admin));
        assert!(!PermissionLevel::Admin.can_manage(&PermissionLevel::Owner));
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            PermissionLevel::from_str("read").unwrap(),
            PermissionLevel::Read
        );
        assert_eq!(
            PermissionLevel::from_str("WRITE").unwrap(),
            PermissionLevel::Write
        );
        assert!(PermissionLevel::from_str("invalid").is_err());
    }
}

