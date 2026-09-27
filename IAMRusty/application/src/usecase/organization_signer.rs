//! Organization signer façade (ADR-0306) — configure / test / rotate / disable.
//!
//! Returns [`DomainError`] (not HTTP status) so InProcess adapters need no Axum.

use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::signing_key::{
    opaque_kid, SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
    FORBIDDEN_TRANSIT_KEY_NAME,
};
use iam_domain::entity::token::Jwk;
use iam_domain::error::DomainError;
use iam_domain::port::repository::SigningKeyRegistry;
use iam_domain::port::{OrganizationSignerProbe, OrganizationSignerRotator};
use std::sync::Arc;
use uuid::Uuid;

/// Message used when an issuer URL is already bound to another organization.
pub const ISSUER_OWNED_BY_OTHER_ORGANIZATION: &str =
    "organization issuer owned by another organization";

/// Message when no active (or any) organization signing key exists.
pub const NO_ACTIVE_ORGANIZATION_SIGNING_KEY: &str = "no active organization signing key";

const MAX_PUBLIC_KEY_BYTES: usize = 16 * 1024;

/// Input for ConfigureOrganizationSigner (IAM-owned DTO — not hive-domain).
#[derive(Debug, Clone)]
pub struct ConfigureOrganizationSignerInput {
    pub provider_type: String,
    pub provider_key_ref: String,
    pub credential_ref: Option<String>,
    pub public_key: String,
    /// Trusted from Hive (Admin already checked). Bound to `Organization.slug`.
    pub org_slug: String,
}

/// Metadata returned by signer ops — never contains private key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationSignerResult {
    pub signing_profile_id: Uuid,
    pub kid: String,
    pub status: String,
    pub issuer: String,
}

impl OrganizationSignerResult {
    fn from_key(key: &SigningKey) -> Self {
        Self {
            signing_profile_id: key.id,
            kid: key.kid.clone(),
            status: String::from(&key.status),
            issuer: key.issuer.clone(),
        }
    }
}

/// Application façade for organization signer lifecycle (ADR-0306).
///
/// No Admin / OpenFGA check here — Hive HTTP owns AuthZ. No internal token check
/// here — IAM HTTP handlers enforce `x-iam-internal-token` on the HTTP path only.
#[async_trait]
pub trait OrganizationSignerFacade: Send + Sync {
    async fn configure(
        &self,
        organization_id: Uuid,
        request: &ConfigureOrganizationSignerInput,
    ) -> Result<OrganizationSignerResult, DomainError>;

    async fn test(&self, organization_id: Uuid) -> Result<OrganizationSignerResult, DomainError>;

    async fn rotate(&self, organization_id: Uuid) -> Result<OrganizationSignerResult, DomainError>;

    async fn disable(&self, organization_id: Uuid)
        -> Result<OrganizationSignerResult, DomainError>;
}

/// Default façade wired from IAM setup / `SignerRouteContext`.
pub struct OrganizationSignerFacadeImpl {
    registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
    public_base_url: String,
    probe: Arc<dyn OrganizationSignerProbe>,
    rotator: Arc<dyn OrganizationSignerRotator>,
}

impl OrganizationSignerFacadeImpl {
    #[must_use]
    pub fn new(
        registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
        public_base_url: impl Into<String>,
        probe: Arc<dyn OrganizationSignerProbe>,
        rotator: Arc<dyn OrganizationSignerRotator>,
    ) -> Self {
        Self {
            registry,
            public_base_url: public_base_url.into(),
            probe,
            rotator,
        }
    }
}

#[async_trait]
impl OrganizationSignerFacade for OrganizationSignerFacadeImpl {
    async fn configure(
        &self,
        organization_id: Uuid,
        request: &ConfigureOrganizationSignerInput,
    ) -> Result<OrganizationSignerResult, DomainError> {
        if unsupported_cloud(&request.provider_type) {
            return Err(DomainError::ProviderNotSupported(
                request.provider_type.clone(),
            ));
        }
        let provider_type: SigningProviderType = request
            .provider_type
            .parse()
            .map_err(|_| DomainError::ProviderNotSupported(request.provider_type.clone()))?;
        if provider_type.is_unsupported_cloud_byokms() {
            return Err(DomainError::ProviderNotSupported(
                request.provider_type.clone(),
            ));
        }
        if request.provider_key_ref == FORBIDDEN_TRANSIT_KEY_NAME {
            return Err(DomainError::BusinessRuleViolation(format!(
                "Transit key name {FORBIDDEN_TRANSIT_KEY_NAME} is forbidden"
            )));
        }
        if provider_type == SigningProviderType::PemFile {
            require_org_scoped_pem_ref(organization_id, &request.provider_key_ref)?;
        }
        require_rsa_public_pem(&request.public_key)?;

        let issuer = org_issuer(&self.public_base_url, &request.org_slug);
        let existing = self.registry.find_by_issuer(&issuer).await?;
        if issuer_owned_by_other_organization(&existing, organization_id) {
            return Err(DomainError::BusinessRuleViolation(
                ISSUER_OWNED_BY_OTHER_ORGANIZATION.to_string(),
            ));
        }

        let org_keys = self.registry.find_by_organization(organization_id).await?;
        for mut previous in org_keys {
            if previous.status.can_sign() {
                previous.status = SigningKeyStatus::Retiring;
                previous.updated_at = Utc::now();
                self.registry.update(&previous).await?;
            }
        }

        let now = Utc::now();
        let kid = opaque_kid();
        let key = SigningKey {
            id: Uuid::new_v4(),
            kid: kid.clone(),
            algorithm: "RS256".to_string(),
            trust_scope: TrustScope::Organization,
            issuer: issuer.clone(),
            provider_type,
            provider_key_ref: request.provider_key_ref.clone(),
            credential_ref: request.credential_ref.clone(),
            public_key: request.public_key.clone(),
            status: SigningKeyStatus::Active,
            organization_id: Some(organization_id),
            created_at: now,
            updated_at: now,
        };
        self.registry.insert(&key).await?;

        Ok(OrganizationSignerResult {
            signing_profile_id: key.id,
            kid,
            status: String::from(&key.status),
            issuer,
        })
    }

    async fn test(&self, organization_id: Uuid) -> Result<OrganizationSignerResult, DomainError> {
        let keys = self.registry.find_by_organization(organization_id).await?;
        let key = keys
            .into_iter()
            .find(|k| k.status.can_sign())
            .ok_or_else(|| {
                DomainError::AuthorizationError(NO_ACTIVE_ORGANIZATION_SIGNING_KEY.to_string())
            })?;
        self.probe.challenge(&key).await?;
        Ok(OrganizationSignerResult::from_key(&key))
    }

    async fn rotate(&self, organization_id: Uuid) -> Result<OrganizationSignerResult, DomainError> {
        let key = self.rotator.rotate(organization_id).await?;
        Ok(OrganizationSignerResult::from_key(&key))
    }

    async fn disable(
        &self,
        organization_id: Uuid,
    ) -> Result<OrganizationSignerResult, DomainError> {
        let keys = self.registry.find_by_organization(organization_id).await?;
        let mut last = None;
        for mut key in keys {
            key.status = SigningKeyStatus::Revoked;
            key.updated_at = Utc::now();
            self.registry.update(&key).await?;
            last = Some(key);
        }
        let key = last.ok_or_else(|| {
            DomainError::AuthorizationError(NO_ACTIVE_ORGANIZATION_SIGNING_KEY.to_string())
        })?;
        Ok(OrganizationSignerResult::from_key(&key))
    }
}

fn unsupported_cloud(provider_type: &str) -> bool {
    matches!(
        provider_type,
        "aws_kms" | "gcp_kms" | "azure_key_vault" | "AwsKms" | "GcpKms" | "AzureKeyVault"
    )
}

fn org_issuer(public_base_url: &str, org_slug: &str) -> String {
    format!(
        "{}/iam/orgs/{org_slug}",
        public_base_url.trim_end_matches('/')
    )
}

fn require_rsa_public_pem(public_key: &str) -> Result<(), DomainError> {
    if public_key.len() > MAX_PUBLIC_KEY_BYTES {
        return Err(DomainError::BusinessRuleViolation(
            "public_key exceeds maximum size".into(),
        ));
    }
    Jwk::from_rsa_pem(public_key, "validate", "validate")
        .map(|_| ())
        .map_err(|_| {
            DomainError::BusinessRuleViolation("public_key must be a valid RSA PEM".into())
        })
}

fn issuer_owned_by_other_organization(keys: &[SigningKey], org_id: Uuid) -> bool {
    keys.iter()
        .any(|k| k.organization_id.is_some_and(|id| id != org_id))
}

/// PEM `provider_key_ref` must be a relative path under `{org_id}/` (no `..`).
///
/// # Errors
///
/// Returns [`DomainError::BusinessRuleViolation`] when the ref is empty, absolute,
/// contains `..`, or is not under `{org_id}/`.
pub fn require_org_scoped_pem_ref(org_id: Uuid, provider_key_ref: &str) -> Result<(), DomainError> {
    let requested = std::path::Path::new(provider_key_ref);
    if provider_key_ref.trim().is_empty() || requested.is_absolute() {
        return Err(DomainError::BusinessRuleViolation(
            "PEM provider_key_ref must be a non-empty relative path".into(),
        ));
    }
    if requested
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(DomainError::BusinessRuleViolation(
            "PEM path must not contain '..'".into(),
        ));
    }

    let org = org_id.to_string();
    let mut components = requested.components();
    let under_org = matches!(
        components.next(),
        Some(std::path::Component::Normal(first)) if first == org.as_str()
    ) && components.next().is_some();
    if !under_org {
        return Err(DomainError::BusinessRuleViolation(
            "PEM provider_key_ref must be under {org_id}/".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_pem_ref_must_stay_under_org_subtree() {
        let org_id = Uuid::new_v4();
        let other = Uuid::new_v4();
        assert!(require_org_scoped_pem_ref(org_id, "test-platform.pem").is_err());
        assert!(require_org_scoped_pem_ref(org_id, &format!("{other}/kid.pem")).is_err());
        assert!(require_org_scoped_pem_ref(org_id, "../test-platform.pem").is_err());
        assert!(require_org_scoped_pem_ref(org_id, &format!("{org_id}/kid.pem")).is_ok());
    }
}
