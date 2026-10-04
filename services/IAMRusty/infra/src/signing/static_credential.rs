//! StaticCredential WorkloadIdentity adapter (config / OpenBao ref map).

use async_trait::async_trait;
use iam_domain::error::DomainError;
use iam_domain::port::{WorkloadCredential, WorkloadIdentity};
use std::collections::HashMap;
use std::sync::Arc;

/// Resolves named credential refs from an in-memory map (config or OpenBao-injected secrets).
///
/// Never stores secrets in Hive DB or domain events.
#[derive(Clone, Default)]
pub struct StaticCredential {
    secrets: Arc<HashMap<String, String>>,
}

impl std::fmt::Debug for StaticCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticCredential")
            .field("entries", &self.secrets.len())
            .finish_non_exhaustive()
    }
}

impl StaticCredential {
    /// Build from an explicit map of credential_ref → secret value.
    #[must_use]
    pub fn new(secrets: HashMap<String, String>) -> Self {
        Self {
            secrets: Arc::new(secrets),
        }
    }

    /// Single-entry helper. Empty secrets (after trim) are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when `secret` is empty after trim.
    pub fn from_pair(
        credential_ref: impl Into<String>,
        secret: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let secret = secret.into();
        if secret.trim().is_empty() {
            return Err(DomainError::external_service_error(
                "workload_identity",
                "credential secret must not be empty",
            ));
        }
        let mut map = HashMap::new();
        map.insert(credential_ref.into(), secret);
        Ok(Self::new(map))
    }
}

#[async_trait]
impl WorkloadIdentity for StaticCredential {
    async fn resolve(&self, credential_ref: &str) -> Result<WorkloadCredential, DomainError> {
        self.secrets
            .get(credential_ref)
            .cloned()
            .map(|secret| WorkloadCredential { secret })
            .ok_or_else(|| {
                DomainError::external_service_error(
                    "workload_identity",
                    "unknown credential reference",
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn static_credential_resolves_and_misses() {
        let wi = StaticCredential::from_pair("openbao-token", "s.secret").expect("pair");
        let ok = wi.resolve("openbao-token").await.expect("resolve");
        assert_eq!(ok.secret, "s.secret");
        assert!(wi.resolve("missing").await.is_err());
    }

    #[test]
    fn static_credential_rejects_empty_secret() {
        assert!(StaticCredential::from_pair("openbao-token", "  ").is_err());
        assert!(StaticCredential::from_pair("openbao-token", "").is_err());
    }

    #[test]
    fn debug_redacts_entire_credential_map() {
        let wi = StaticCredential::from_pair("unit-ref", "unit-secret").unwrap();
        let debug = format!("{wi:?}");
        assert!(!debug.contains("unit-secret"));
        assert!(!debug.contains("unit-ref"));
    }
}
