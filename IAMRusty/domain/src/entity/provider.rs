use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use idp_connect_contract::{ProviderTokens, ProviderUserProfile};

/// Typed IdP slug (`github`, `gitlab`, `huggingface`, …).
///
/// Parse accepts ASCII letters only (`^[a-zA-Z]+$`), then ASCII case-folds.
/// Canon is `^[a-z]+$`, length 1–50. Not `Copy`: the inner slug is owned.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Provider(String);

/// Rejected IdP slug (illegal charset, empty, or longer than 50).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid OAuth2 provider slug")]
pub struct ProviderParseError;

impl Provider {
    /// Parse an IdP slug: letters-only, ASCII case-fold, length 1–50.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderParseError`] when `raw` is empty, longer than 50
    /// characters, or contains a non-letter (digit, hyphen, unicode, …).
    pub fn parse_slug(raw: &str) -> Result<Self, ProviderParseError> {
        if raw.is_empty() || raw.len() > 50 {
            return Err(ProviderParseError);
        }
        if !raw.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err(ProviderParseError);
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    /// Canonical lowercase slug.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Provider {
    type Err = ProviderParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_slug(s)
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Provider {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Provider {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse_slug(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_folds_github_case() {
        let provider = Provider::parse_slug("GitHub").expect("letters");
        assert_eq!(provider.as_str(), "github");
        assert_eq!(provider.to_string(), "github");
        assert_eq!(provider, "github".parse().expect("canon"));
    }

    #[test]
    fn parse_accepts_huggingface() {
        let provider: Provider = "huggingface".parse().expect("letters-only slug");
        assert_eq!(provider.as_str(), "huggingface");
    }

    #[test]
    fn parse_rejects_hyphen_digit_empty_and_overlong() {
        assert!("hugging-face".parse::<Provider>().is_err());
        assert!("github2".parse::<Provider>().is_err());
        assert!("".parse::<Provider>().is_err());
        assert!("a".repeat(51).parse::<Provider>().is_err());
        assert!("a".repeat(50).parse::<Provider>().is_ok());
    }

    #[test]
    fn serde_roundtrip_is_lowercase_slug() {
        let provider = Provider::parse_slug("GitLab").expect("letters");
        let json = serde_json::to_string(&provider).expect("serialize");
        assert_eq!(json, "\"gitlab\"");
        let back: Provider = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, provider);
    }

    #[test]
    fn serde_accepts_folded_input() {
        let provider: Provider = serde_json::from_str("\"GitHub\"").expect("fold");
        assert_eq!(provider.as_str(), "github");
    }

    #[test]
    fn serde_rejects_illegal_slug() {
        assert!(serde_json::from_str::<Provider>("\"bit-bucket\"").is_err());
    }
}
