//! Organization signer façade (ADR-0306) — configure / test / rotate / disable.
//!
//! Returns [`DomainError`] (not HTTP status) so InProcess adapters need no Axum.

use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::signing_key::{
    opaque_kid, same_effective_signing_binding, SigningKey, SigningKeyPreparation,
    SigningKeyStatus, SigningProviderType, SigningScope, TrustScope, FORBIDDEN_TRANSIT_KEY_NAME,
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
    pub provider_key_version: Option<u32>,
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
        if provider_type == SigningProviderType::OpenBaoTransit {
            iam_domain::entity::signing_key::require_transit_key_version(
                request.provider_key_version,
            )?;
            iam_domain::entity::signing_key::require_org_transit_binding(
                organization_id,
                &request.provider_key_ref,
                request.credential_ref.as_deref(),
            )?;
        }
        require_rsa_public_pem(&request.public_key)?;

        let issuer = org_issuer(&self.public_base_url, &request.org_slug);
        // Scope revision is captured before public retrieval / PoP. It remains
        // meaningful even when disable revoked the last row or affected no rows.
        let before = self
            .registry
            .signing_scope_snapshot(&SigningScope::organization(organization_id))
            .await?;
        let existing = self.registry.find_by_issuer(&issuer).await?;
        if issuer_owned_by_other_organization(&existing, organization_id) {
            return Err(DomainError::BusinessRuleViolation(
                ISSUER_OWNED_BY_OTHER_ORGANIZATION.to_string(),
            ));
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
            provider_key_version: request.provider_key_version,
            credential_ref: request.credential_ref.clone(),
            public_key: request.public_key.clone(),
            status: SigningKeyStatus::Pending,
            organization_id: Some(organization_id),
            created_at: now,
            updated_at: now,
        };
        // Validate backend/material binding before touching the published epoch.
        self.probe.validate_configuration(&key)?;
        let key = if let Some(active) = before
            .active
            .as_ref()
            .filter(|active| same_effective_signing_binding(active, &key))
        {
            if !self.registry.confirm_active_for_emission(active).await? {
                return Err(iam_domain::entity::signing_key::admission_denied(
                    iam_domain::entity::signing_key::SigningKeyAdmissionReason::EpochConflict,
                ));
            }
            active.clone()
        } else if let Some(pending) = before
            .pending
            .as_ref()
            .filter(|pending| same_effective_signing_binding(&pending.key, &key))
        {
            // Durable proof/publication already exists. No second provider Rotate,
            // admission, or probe whose outcome could replace this pinned binding.
            self.registry.promote_signing_key(pending).await?
        } else {
            let proof = self.probe.prove(key, before).await?;
            match self.registry.prepare_signing_key(proof).await? {
                SigningKeyPreparation::Unchanged(key) => key,
                SigningKeyPreparation::Pending(pending) => {
                    self.registry.promote_signing_key(&pending).await?
                }
            }
        };

        Ok(OrganizationSignerResult {
            signing_profile_id: key.id,
            kid: key.kid,
            status: String::from(&key.status),
            issuer: key.issuer,
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
        if !self.registry.confirm_active_for_emission(&key).await? {
            return Err(iam_domain::entity::signing_key::admission_denied(
                iam_domain::entity::signing_key::SigningKeyAdmissionReason::EpochConflict,
            ));
        }
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
        let keys = self
            .registry
            .revoke_organization_keys(organization_id)
            .await?;
        let key = keys.into_iter().last().ok_or_else(|| {
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

    struct AtomicFailureRegistry {
        previous: SigningKey,
        attempted: std::sync::atomic::AtomicBool,
        get_fence: Option<std::sync::atomic::AtomicBool>,
    }
    #[async_trait]
    impl SigningKeyRegistry for AtomicFailureRegistry {
        type Error = DomainError;
        async fn signing_scope_snapshot(
            &self,
            scope: &SigningScope,
        ) -> Result<iam_domain::entity::signing_key::SigningScopeSnapshot, DomainError> {
            self.attempted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(iam_domain::entity::signing_key::SigningScopeSnapshot {
                scope: scope.clone(),
                revision: 1,
                active: Some(self.previous.clone()),
                pending: None,
            })
        }
        async fn prepare_signing_key(
            &self,
            _: iam_domain::entity::signing_key::ProbedSigningKey,
        ) -> Result<SigningKeyPreparation, DomainError> {
            self.attempted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Err(DomainError::RepositoryError(
                "injected Pending admission failure".into(),
            ))
        }
        async fn jwks_publication_snapshot(
            &self,
        ) -> Result<iam_domain::entity::signing_key::SigningKeyPublicationSnapshot, DomainError>
        {
            panic!("configure must not use publication snapshot")
        }
        async fn confirm_active_for_emission(&self, _: &SigningKey) -> Result<bool, DomainError> {
            let allowed = self
                .get_fence
                .as_ref()
                .expect("configuration must not invoke an emission fence");
            self.attempted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(allowed.load(std::sync::atomic::Ordering::SeqCst))
        }
        async fn insert(&self, _: &SigningKey) -> Result<(), DomainError> {
            panic!("non-atomic insert")
        }
        async fn update(&self, _: &SigningKey) -> Result<(), DomainError> {
            panic!("non-atomic retirement")
        }
        async fn replace_active_organization_key(
            &self,
            _: &SigningKey,
            _: Option<&str>,
        ) -> Result<SigningKey, DomainError> {
            self.attempted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Err(DomainError::RepositoryError(
                "injected insert failure".into(),
            ))
        }
        async fn revoke_organization_keys(&self, _: Uuid) -> Result<Vec<SigningKey>, DomainError> {
            unreachable!()
        }
        async fn find_by_kid(&self, _: &str) -> Result<Option<SigningKey>, DomainError> {
            Ok(Some(self.previous.clone()))
        }
        async fn find_active_platform_key(&self) -> Result<Option<SigningKey>, DomainError> {
            Ok(None)
        }
        async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, DomainError> {
            Ok(vec![self.previous.clone()])
        }
        async fn find_by_organization(&self, _: Uuid) -> Result<Vec<SigningKey>, DomainError> {
            Ok(vec![self.previous.clone()])
        }
        async fn find_by_issuer(&self, _: &str) -> Result<Vec<SigningKey>, DomainError> {
            self.attempted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![self.previous.clone()])
        }
    }
    struct Probe;
    #[async_trait]
    impl OrganizationSignerProbe for Probe {
        async fn challenge(&self, _: &SigningKey) -> Result<(), DomainError> {
            Ok(())
        }
    }
    struct Rotator;
    #[async_trait]
    impl OrganizationSignerRotator for Rotator {
        async fn rotate(&self, _: Uuid) -> Result<SigningKey, DomainError> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn transit_missing_and_zero_version_fail_before_registry_probe_or_credentials() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct CountingProbe(AtomicUsize);
        #[async_trait]
        impl OrganizationSignerProbe for CountingProbe {
            async fn challenge(&self, _: &SigningKey) -> Result<(), DomainError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Err(DomainError::external_service_error(
                    "provider",
                    "must-not-run",
                ))
            }
        }
        let org = Uuid::new_v4();
        let now = Utc::now();
        let registry = Arc::new(AtomicFailureRegistry {
            previous: SigningKey {
                id: Uuid::new_v4(),
                kid: opaque_kid(),
                algorithm: "RS256".into(),
                trust_scope: TrustScope::Organization,
                issuer: "https://iam.example/iam/orgs/acme".into(),
                provider_type: SigningProviderType::OpenBaoTransit,
                provider_key_ref: format!("org-{org}-key"),
                provider_key_version: Some(7),
                credential_ref: Some(format!("org-{org}-credential")),
                public_key: include_str!("../../../config/keys/test-platform.pub").into(),
                status: SigningKeyStatus::Active,
                organization_id: Some(org),
                created_at: now,
                updated_at: now,
            },
            attempted: std::sync::atomic::AtomicBool::new(false),
            get_fence: None,
        });
        let probe = Arc::new(CountingProbe(AtomicUsize::new(0)));
        let facade = OrganizationSignerFacadeImpl::new(
            registry.clone(),
            "https://iam.example",
            probe.clone(),
            Arc::new(Rotator),
        );
        for version in [None, Some(0)] {
            let err = facade
                .configure(
                    org,
                    &ConfigureOrganizationSignerInput {
                        provider_type: "openbao_transit".into(),
                        provider_key_ref: format!("org-{org}-key"),
                        provider_key_version: version,
                        credential_ref: Some(format!("org-{org}-credential")),
                        public_key: registry.previous.public_key.clone(),
                        org_slug: "acme".into(),
                    },
                )
                .await
                .unwrap_err();
            assert!(matches!(err, DomainError::InvalidSigningKeyMaterial));
            assert!(!registry.attempted.load(Ordering::SeqCst));
            assert_eq!(probe.0.load(Ordering::SeqCst), 0);
        }
    }
    #[tokio::test]
    async fn failed_configure_never_retires_previous_via_separate_update() {
        let org = Uuid::new_v4();
        let now = Utc::now();
        let public = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(AtomicFailureRegistry {
            previous: SigningKey {
                id: Uuid::new_v4(),
                kid: "previous".into(),
                algorithm: "RS256".into(),
                trust_scope: TrustScope::Organization,
                issuer: "https://iam.example/iam/orgs/acme".into(),
                provider_type: SigningProviderType::PemFile,
                provider_key_ref: format!("{org}/previous.pem"),
                provider_key_version: None,
                credential_ref: None,
                public_key: public.into(),
                status: SigningKeyStatus::Active,
                organization_id: Some(org),
                created_at: now,
                updated_at: now,
            },
            attempted: std::sync::atomic::AtomicBool::new(false),
            get_fence: None,
        });
        let facade = OrganizationSignerFacadeImpl::new(
            registry.clone(),
            "https://iam.example",
            Arc::new(Probe),
            Arc::new(Rotator),
        );
        let result = facade
            .configure(
                org,
                &ConfigureOrganizationSignerInput {
                    provider_type: "pem_file".into(),
                    provider_key_ref: format!("{org}/new.pem"),
                    provider_key_version: None,
                    credential_ref: None,
                    public_key: public.into(),
                    org_slug: "acme".into(),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(registry.attempted.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(registry.previous.status, SigningKeyStatus::Active);
    }

    #[tokio::test]
    async fn get_signer_refuses_a_revocation_committed_during_probe_before_returning_to_hive() {
        struct RevokeDuringProbe(Arc<AtomicFailureRegistry>);
        #[async_trait]
        impl OrganizationSignerProbe for RevokeDuringProbe {
            async fn challenge(&self, _: &SigningKey) -> Result<(), DomainError> {
                self.0
                    .get_fence
                    .as_ref()
                    .unwrap()
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }
        let org = Uuid::new_v4();
        let now = Utc::now();
        let key = SigningKey {
            id: Uuid::new_v4(),
            kid: opaque_kid(),
            algorithm: "RS256".into(),
            trust_scope: TrustScope::Organization,
            issuer: org_issuer("https://platform.example", "fixture"),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: format!("{org}/fixture.pem"),
            provider_key_version: None,
            credential_ref: None,
            public_key: include_str!("../../../config/keys/test-platform.pub").into(),
            status: SigningKeyStatus::Active,
            organization_id: Some(org),
            created_at: now,
            updated_at: now,
        };
        let registry = Arc::new(AtomicFailureRegistry {
            previous: key.clone(),
            attempted: std::sync::atomic::AtomicBool::new(false),
            get_fence: Some(std::sync::atomic::AtomicBool::new(true)),
        });
        let positive = OrganizationSignerFacadeImpl::new(
            registry.clone(),
            "https://platform.example",
            Arc::new(Probe),
            Arc::new(Rotator),
        );
        assert_eq!(positive.test(org).await.unwrap().kid, key.kid);
        assert!(registry
            .attempted
            .swap(false, std::sync::atomic::Ordering::SeqCst));
        let racing = OrganizationSignerFacadeImpl::new(
            registry.clone(),
            "https://platform.example",
            Arc::new(RevokeDuringProbe(registry.clone())),
            Arc::new(Rotator),
        );
        assert!(matches!(
            racing.test(org).await,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: iam_domain::entity::signing_key::SigningKeyAdmissionReason::EpochConflict,
                ..
            })
        ));
        assert!(
            registry.attempted.load(std::sync::atomic::Ordering::SeqCst),
            "the post-probe primary fence must run before returning a profile"
        );
        assert_eq!(
            registry.previous, key,
            "the initial object intentionally remains stale"
        );
    }
}
