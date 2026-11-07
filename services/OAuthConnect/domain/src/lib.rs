//! Vendor-neutral declarations shared by configuration and pure projection.

use serde::{Deserialize, Serialize};

/// Identifier of a configured GET JSON API, not an endpoint or executable query.
pub type ApiId = String;

/// One claim selected from one response in the current completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimSource {
    pub api: ApiId,
    pub json_path: String,
}

/// Required OAuth mappings, plus the existing optional avatar mapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OAuthClaims {
    pub sub: ClaimSource,
    pub email: ClaimSource,
    pub email_verified: ClaimSource,
    pub preferred_username: ClaimSource,
    #[serde(default)]
    pub picture: Option<ClaimSource>,
}
