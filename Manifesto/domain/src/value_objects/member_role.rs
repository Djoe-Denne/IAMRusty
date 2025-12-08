use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Read,
    Write,
    Admin,
    Owner,
}

impl MemberRole {
    /// Check if this role can manage another role
    /// (e.g., Admin can manage Write and Read, but not Owner)
    pub fn can_manage(&self, other: &MemberRole) -> bool {
        self > other
    }

    /// Check if this role has at least the permissions of another role
    pub fn has_permission_of(&self, other: &MemberRole) -> bool {
        self >= other
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            "owner" => Ok(Self::Owner),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid member role: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_manage() {
        assert!(MemberRole::Owner.can_manage(&MemberRole::Admin));
        assert!(MemberRole::Owner.can_manage(&MemberRole::Write));
        assert!(MemberRole::Admin.can_manage(&MemberRole::Write));
        assert!(MemberRole::Admin.can_manage(&MemberRole::Read));
        assert!(!MemberRole::Write.can_manage(&MemberRole::Admin));
        assert!(!MemberRole::Admin.can_manage(&MemberRole::Owner));
    }

    #[test]
    fn test_has_permission_of() {
        assert!(MemberRole::Owner.has_permission_of(&MemberRole::Read));
        assert!(MemberRole::Admin.has_permission_of(&MemberRole::Write));
        assert!(MemberRole::Write.has_permission_of(&MemberRole::Write));
        assert!(!MemberRole::Read.has_permission_of(&MemberRole::Write));
    }
}

