//! Vendor-neutral declarations shared by configuration and pure projection.

use serde::{Deserialize, Serialize};

/// Identifier of a configured GET JSON API, not an endpoint or executable query.
pub type ApiId = String;

/// One claim selected from one response in the current completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimSource {
    /// Configured API identifier whose response supplies this claim.
    pub api: ApiId,
    /// JSONPath expression; AST policy is checked by infra, not deserialization.
    pub json_path: String,
}

/// Required OAuth mappings, plus the existing optional avatar mapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OAuthClaims {
    /// External subject, preserved as a nonempty string or normalized from a u64.
    pub sub: ClaimSource,
    /// Optional runtime email; the mapping itself is required.
    pub email: ClaimSource,
    /// Verification boolean paired with the email from the same selected object.
    pub email_verified: ClaimSource,
    /// Descriptive username; absence must not fall back to email or subject.
    pub preferred_username: ClaimSource,
    /// Optional mapping for the existing avatar URL, never downloaded by projection.
    #[serde(default)]
    pub picture: Option<ClaimSource>,
}
