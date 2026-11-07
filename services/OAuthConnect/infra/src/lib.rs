//! Pure projection of already acquired JSON responses; no HTTP or OIDC verifier.

use std::collections::BTreeMap;

pub use idp_connect_contract::ProviderUserProfile;
pub use oauth_connect_domain::{ApiId, ClaimSource, OAuthClaims};
pub use rustycog::core::error::DomainError;
use serde_json::Value;

/// Responses for exactly one completion, keyed by configured API identifier.
pub type ApiResponses = BTreeMap<ApiId, Value>;

/// Opaque validated mapping. Only `compile_mapping` may create this capability.
/// Internal representation may retain the maintained parser's AST in T2.
#[derive(Debug, Clone)]
pub struct CompiledMapping {
    _private: (),
}

/// Validate JSONPath expressions and email/verified structural correlation.
///
/// # Errors
/// Always refuses in T0; parsing/AST policy is deliberately not implemented yet.
pub fn compile_mapping(_claims: &OAuthClaims) -> Result<CompiledMapping, DomainError> {
    Err(DomainError::InvalidInput {
        message: "OAuthConnect mapping compilation not implemented".to_owned(),
    })
}

impl CompiledMapping {
    /// Project the current completion without retaining JSON or accessing a network.
    ///
    /// # Errors
    /// Always refuses in T0; no cardinality, type or correlation check is claimed.
    pub fn project(&self, _responses: &ApiResponses) -> Result<ProviderUserProfile, DomainError> {
        Err(DomainError::InvalidInput {
            message: "OAuthConnect profile projection not implemented".to_owned(),
        })
    }
}

/// Compile and project in one pure operation, convenient for in-memory callers.
///
/// # Errors
/// Returns compilation/projection errors without exposing response values.
pub fn project_profile(
    claims: &OAuthClaims,
    responses: &ApiResponses,
) -> Result<ProviderUserProfile, DomainError> {
    compile_mapping(claims)?.project(responses)
}
