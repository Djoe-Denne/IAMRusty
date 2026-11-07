//! OAuth transport and pure projection; no OIDC verifier.

use std::collections::BTreeMap;
use std::fmt;

pub use idp_connect_contract::ProviderUserProfile;
pub use oauth_connect_domain::{ApiId, ClaimSource, OAuthClaims};
pub use rustycog::core::error::DomainError;
use serde_json::Value;

mod budget;
mod json_path;
mod oauth;
mod transport;

pub use oauth::OAuthClient;
pub use transport::{HTTP_BODY_LIMIT, HTTP_CONNECT_TIMEOUT, HTTP_REQUEST_TIMEOUT};

use json_path::CompiledClaim;

/// Responses for exactly one completion, keyed by configured API identifier.
pub type ApiResponses = BTreeMap<ApiId, Value>;

/// Opaque validated mapping. Only `compile_mapping` may create this capability.
/// Retains only API identifiers and validated `JSONPath` ASTs, never response data.
#[derive(Clone)]
pub struct CompiledMapping {
    sub: CompiledClaim,
    email: CompiledClaim,
    email_verified: CompiledClaim,
    preferred_username: CompiledClaim,
    picture: Option<CompiledClaim>,
}

impl fmt::Debug for CompiledMapping {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompiledMapping")
            .finish_non_exhaustive()
    }
}

/// Validate `JSONPath` expressions and email/verified structural correlation.
///
/// # Errors
/// Refuses invalid, unsupported or excessive expressions and disjoint email sources.
/// Diagnostics never include expressions or their literals.
pub fn compile_mapping(claims: &OAuthClaims) -> Result<CompiledMapping, DomainError> {
    let mapping = CompiledMapping {
        sub: CompiledClaim::compile(&claims.sub)?,
        email: CompiledClaim::compile(&claims.email)?,
        email_verified: CompiledClaim::compile(&claims.email_verified)?,
        preferred_username: CompiledClaim::compile(&claims.preferred_username)?,
        picture: claims
            .picture
            .as_ref()
            .map(CompiledClaim::compile)
            .transpose()?,
    };
    mapping
        .email
        .ensure_compatible_parent(&mapping.email_verified)?;
    Ok(mapping)
}

impl CompiledMapping {
    /// Project the current completion without retaining JSON or accessing a network.
    ///
    /// # Errors
    /// Refuses missing or excessive API responses, excessive estimated evaluation
    /// cost, invalid types/cardinalities and email pairs from different objects.
    /// Diagnostics contain no response values.
    pub fn project(&self, responses: &ApiResponses) -> Result<ProviderUserProfile, DomainError> {
        let admission = budget::admit(self, responses)?;
        let sub = self.sub.select(&admission)?;
        let email = self.email.select(&admission)?;
        let verified = self.email_verified.select(&admission)?;
        let username = self.preferred_username.select(&admission)?;
        let picture = self
            .picture
            .as_ref()
            .map(|claim| claim.select(&admission))
            .transpose()?
            .flatten();

        let id = subject(sub.as_ref().map(|selected| selected.val))?;
        let email_value = optional_string(email.as_ref().map(|selected| selected.val), true)?;
        // Validate even when email is absent: absence must not hide malformed data.
        let verified_value = optional_boolean(verified.as_ref().map(|selected| selected.val))?;
        if let (Some(email), Some(verified)) = (&email, &verified) {
            self.email.ensure_same_parent(email, verified, &admission)?;
        }
        Ok(ProviderUserProfile {
            id,
            username: optional_string(username.as_ref().map(|selected| selected.val), false)?
                .unwrap_or_default(),
            email_verified: email_value.is_some() && verified_value,
            email: email_value,
            avatar_url: optional_string(picture.as_ref().map(|selected| selected.val), false)?,
        })
    }

    fn claims(&self) -> impl Iterator<Item = &CompiledClaim> {
        [
            &self.sub,
            &self.email,
            &self.email_verified,
            &self.preferred_username,
        ]
        .into_iter()
        .chain(self.picture.iter())
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

fn invalid_mapping() -> DomainError {
    DomainError::InvalidInput {
        message: "Invalid OAuthConnect claim mapping or profile".to_owned(),
    }
}

fn subject(value: Option<&Value>) -> Result<String, DomainError> {
    match value {
        Some(Value::String(value)) if !value.is_empty() => Ok(value.clone()),
        Some(Value::Number(value)) => value
            .as_u64()
            .map(|value| value.to_string())
            .ok_or_else(invalid_mapping),
        _ => Err(invalid_mapping()),
    }
}

fn optional_string(value: Option<&Value>, nonempty: bool) -> Result<Option<String>, DomainError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !nonempty || !value.is_empty() => Ok(Some(value.clone())),
        _ => Err(invalid_mapping()),
    }
}

fn optional_boolean(value: Option<&Value>) -> Result<bool, DomainError> {
    match value {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        _ => Err(invalid_mapping()),
    }
}
