use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Private,
    Internal,
    Public,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Internal => "internal",
            Self::Public => "public",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "private" => Ok(Self::Private),
            "internal" => Ok(Self::Internal),
            "public" => Ok(Self::Public),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid visibility: {}",
                s
            ))),
        }
    }

    /// Check if this visibility allows public access
    pub fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

