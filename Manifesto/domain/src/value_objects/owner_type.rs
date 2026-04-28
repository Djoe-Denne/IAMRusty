use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnerType {
    Personal,
    Organization,
}

impl OwnerType {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Organization => "organization",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "personal" => Ok(Self::Personal),
            "organization" => Ok(Self::Organization),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid owner type: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for OwnerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
