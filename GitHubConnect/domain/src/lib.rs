//! Domain types for GitHub Connect.

use thiserror::Error;

/// Domain-level errors for GitHub Connect.
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    /// `OAuth2` configuration or URL error (never includes secrets or tokens).
    #[error("OAuth2 error: {0}")]
    OAuth2Error(String),
}

/// Exact string match of `redirect_uri` against the configured allowlist.
#[must_use]
pub fn redirect_uri_allowed(redirect_uri: &str, allowlist: &[String]) -> bool {
    allowlist.iter().any(|allowed| allowed == redirect_uri)
}

#[cfg(test)]
mod tests {
    use super::redirect_uri_allowed;

    #[test]
    fn allowlist_is_exact_string_match() {
        let allowlist = vec!["https://app.example/cb".to_owned()];
        assert!(redirect_uri_allowed("https://app.example/cb", &allowlist));
        assert!(!redirect_uri_allowed("https://app.example/cb/", &allowlist));
        assert!(!redirect_uri_allowed("https://evil.example/cb", &allowlist));
    }
}
