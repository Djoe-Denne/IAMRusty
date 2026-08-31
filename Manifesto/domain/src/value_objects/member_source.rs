use rustycog::core::error::DomainError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberSource {
    Direct,
    OrgCascade,
    Invitation,
    ThirdPartySync,
}

impl MemberSource {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::OrgCascade => "org_cascade",
            Self::Invitation => "invitation",
            Self::ThirdPartySync => "third_party_sync",
        }
    }
}

impl FromStr for MemberSource {
    type Err = DomainError;

    /// Parse a member source from a string.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `s` is not a recognized member source.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(Self::Direct),
            "org_cascade" => Ok(Self::OrgCascade),
            "invitation" => Ok(Self::Invitation),
            "third_party_sync" => Ok(Self::ThirdPartySync),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid member source: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for MemberSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
