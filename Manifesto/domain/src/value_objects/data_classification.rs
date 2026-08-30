use rustycog::core::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

impl DataClassification {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
            Self::Restricted => "restricted",
        }
    }

    /// Parse a data classification from a string.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `s` is not a recognized data classification.
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "public" => Ok(Self::Public),
            "internal" => Ok(Self::Internal),
            "confidential" => Ok(Self::Confidential),
            "restricted" => Ok(Self::Restricted),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid data classification: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for DataClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
