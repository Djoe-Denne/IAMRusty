//! SigningProvider, WorkloadIdentity, and OrganizationSignerProbe ports (ADR-0304 / 0306 / 0307).

use async_trait::async_trait;
use uuid::Uuid;

use crate::entity::signing_key::SigningKey;
use crate::error::DomainError;

/// Capabilities advertised by a concrete signing backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SigningCapabilities {
    /// Whether the provider can sign digests without exporting private key material.
    pub sign_digest: bool,
    /// Whether a local/exportable public key PEM is available.
    pub public_key_available: bool,
}

/// Port used by IAM to request cryptographic signatures for access JWTs.
///
/// KMS is only on the login/refresh path. Adapters must never export private keys
/// (OpenBao Transit Sign, PEM local, …). Cloud BYOKMS adapters are unsupported here.
#[async_trait]
pub trait SigningProvider: Send + Sync {
    /// Sign a pre-hashed digest (typically SHA-256 of the JWS signing input).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the backend rejects the request or is unsupported.
    async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError>;

    /// Return the public key PEM used for JWKS publication, when available.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the public key cannot be retrieved.
    async fn public_key(&self) -> Result<String, DomainError>;

    /// Advertise provider capabilities.
    fn capabilities(&self) -> SigningCapabilities;
}

/// Opaque credential material obtained via [`WorkloadIdentity`].
#[derive(Clone, PartialEq, Eq)]
pub struct WorkloadCredential {
    /// Bearer / token / password material (never logged by callers).
    pub secret: String,
}

impl std::fmt::Debug for WorkloadCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkloadCredential")
            .field("secret", &"[redacted]")
            .finish()
    }
}

/// Port for obtaining s2s / KMS credentials without baking SA key JSON into Hive DB.
///
/// Preference order (ADR-0304 §19 / ADR-0307): OIDC WIF, then X509/mTLS, then
/// [`crate::port::signing`] static secrets (OpenBao / config). SPIFFE/SPIRE is
/// not a required adapter.
#[async_trait]
pub trait WorkloadIdentity: Send + Sync {
    /// Resolve a named credential reference (config key, OpenBao path, …).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the reference cannot be resolved.
    async fn resolve(&self, credential_ref: &str) -> Result<WorkloadCredential, DomainError>;
}

/// Live challenge that IAM can sign-and-verify with an organization key (ADR-0306).
///
/// Hexagonal: HTTP handlers depend on this port; adapters live in iam-infra.
#[async_trait]
pub trait OrganizationSignerProbe: Send + Sync {
    /// Sign a fixed digest with the key's backend and verify it against `key.public_key`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the backend is missing, unsupported, or the
    /// signature does not verify.
    async fn challenge(&self, key: &SigningKey) -> Result<(), DomainError>;
}

/// Rotate an organization signing key with N+1 pending-then-promote (ADR-0304 §13).
#[async_trait]
pub trait OrganizationSignerRotator: Send + Sync {
    /// Mint Pending material, publish it, then promote to Active and retire the previous Active.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when no Active key exists, material cannot be minted, or persistence fails.
    async fn rotate(&self, organization_id: Uuid) -> Result<SigningKey, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::WorkloadCredential;

    #[test]
    fn workload_credential_debug_redacts_secret() {
        let cred = WorkloadCredential {
            secret: "super-secret-token".into(),
        };
        let rendered = format!("{cred:?}");
        assert!(
            rendered.contains("[redacted]"),
            "expected redacted Debug, got {rendered}"
        );
        assert!(
            !rendered.contains("super-secret-token"),
            "secret leaked in Debug: {rendered}"
        );
    }
}
