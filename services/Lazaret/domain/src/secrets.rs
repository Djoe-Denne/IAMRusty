//! Opaque secret references. Plaintext never enters plugin KV.

use async_trait::async_trait;
use thiserror::Error;

/// Failure to resolve an opaque secret reference.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// Reference syntax is not `secret:{path}#{field}`.
    #[error("invalid secret reference")]
    InvalidReference,
    /// Backend miss or transport failure (fail closed).
    #[error("secret resolve failed")]
    ResolveFailed,
}

/// Resolve an opaque reference to bytes. Inject only into a granted operation.
#[async_trait]
pub trait SecretResolver: Send + Sync {
    /// Resolve `secret:{path}#{field}`.
    async fn resolve(&self, reference: &str) -> Result<Vec<u8>, SecretError>;
}

/// Parse `secret:{path}#{field}`.
///
/// Path and field must match `^[A-Za-z0-9/_-]+$`. `.`, `..`, `//`, and `://`
/// are rejected.
///
/// # Errors
///
/// Returns [`SecretError::InvalidReference`] when the shape does not match.
pub fn parse_secret_reference(reference: &str) -> Result<(&str, &str), SecretError> {
    let rest = reference
        .strip_prefix("secret:")
        .ok_or(SecretError::InvalidReference)?;
    let (path, field) = rest.split_once('#').ok_or(SecretError::InvalidReference)?;
    if !is_allowed_secret_token(path) || !is_allowed_secret_token(field) {
        return Err(SecretError::InvalidReference);
    }
    Ok((path, field))
}

fn is_allowed_secret_token(value: &str) -> bool {
    if value.is_empty() || value.contains("//") || value.contains("://") {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-'))
}
