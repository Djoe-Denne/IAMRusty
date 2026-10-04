//! Organization signing-key rotation N+1 (ADR-0304 §13 / ADR-0306).

use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::signing_key::{
    opaque_kid, SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
};
use iam_domain::error::DomainError;
use iam_domain::port::repository::SigningKeyRegistry;
use iam_domain::port::{OrganizationSignerRotator, SigningProvider, WorkloadIdentity};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::RsaPrivateKey;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use super::transit::{TransitSigningProvider, FORBIDDEN_TRANSIT_KEY_NAME};

/// Optional OpenBao Transit client used for org-key mint / probe.
#[derive(Clone)]
pub struct TransitClientConfig {
    pub base_url: String,
    pub workload: Arc<dyn WorkloadIdentity>,
    pub token_ref: String,
}

/// Dependencies for minting and promoting organization signing keys.
#[derive(Clone)]
pub struct RotateContext {
    pub registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
    pub pem_root: PathBuf,
    pub transit: Option<TransitClientConfig>,
}

/// Intermediate state after Pending insert and before Active promote (testable seam).
#[derive(Debug, Clone)]
pub struct PendingRotation {
    pub pending: SigningKey,
    pub previous_active: SigningKey,
}

/// Default rotator wired from setup into HTTP `SignerRouteContext`.
pub struct DefaultOrganizationSignerRotator {
    ctx: RotateContext,
}

impl DefaultOrganizationSignerRotator {
    #[must_use]
    pub fn new(ctx: RotateContext) -> Self {
        Self { ctx }
    }

    #[must_use]
    pub fn context(&self) -> &RotateContext {
        &self.ctx
    }
}

#[async_trait]
impl OrganizationSignerRotator for DefaultOrganizationSignerRotator {
    async fn rotate(&self, organization_id: Uuid) -> Result<SigningKey, DomainError> {
        rotate_organization_signer(&self.ctx, organization_id).await
    }
}

/// Full rotate: mint Pending → promote N+1 Active / retire previous.
///
/// # Errors
///
/// Returns [`DomainError`] when no Active key exists or material / persistence fails.
pub async fn rotate_organization_signer(
    ctx: &RotateContext,
    organization_id: Uuid,
) -> Result<SigningKey, DomainError> {
    let keys = ctx.registry.find_by_organization(organization_id).await?;
    let active = keys
        .into_iter()
        .find(|k| k.status.can_sign())
        .ok_or_else(|| {
            DomainError::AuthorizationError("no active organization signing key".into())
        })?;
    let pending = insert_pending_rotation(ctx, &active).await?;
    promote_pending_rotation(ctx, pending).await
}

/// Mint new material and insert as Pending (JWKS-visible, `can_sign` false).
///
/// # Errors
///
/// Returns [`DomainError`] on crypto / Transit / filesystem / registry failure.
pub async fn insert_pending_rotation(
    ctx: &RotateContext,
    active: &SigningKey,
) -> Result<PendingRotation, DomainError> {
    let kid = opaque_kid();
    if kid == FORBIDDEN_TRANSIT_KEY_NAME || active.provider_key_ref == FORBIDDEN_TRANSIT_KEY_NAME {
        return Err(DomainError::AuthorizationError(
            "Transit key name apparatus-p4-cosign is forbidden".into(),
        ));
    }

    let (provider_key_ref, credential_ref, public_key) =
        mint_material(ctx, active.provider_type.clone(), &kid, active).await?;

    let now = Utc::now();
    let pending = SigningKey {
        id: Uuid::new_v4(),
        kid,
        algorithm: "RS256".to_string(),
        trust_scope: TrustScope::Organization,
        issuer: active.issuer.clone(),
        provider_type: active.provider_type.clone(),
        provider_key_ref,
        credential_ref,
        public_key,
        status: SigningKeyStatus::Pending,
        organization_id: active.organization_id,
        created_at: now,
        updated_at: now,
    };
    ctx.registry.insert(&pending).await?;
    let pending = ctx
        .registry
        .find_by_kid(&pending.kid)
        .await?
        .ok_or(DomainError::InvalidToken)?;
    Ok(PendingRotation {
        pending,
        previous_active: active.clone(),
    })
}

/// Promote Pending → Active and previous Active → Retiring.
///
/// # Errors
///
/// Returns [`DomainError`] on registry update failure.
pub async fn promote_pending_rotation(
    ctx: &RotateContext,
    pending: PendingRotation,
) -> Result<SigningKey, DomainError> {
    let now = Utc::now();
    let mut active = pending.pending;
    active.status = SigningKeyStatus::Active;
    active.updated_at = now;
    ctx.registry
        .replace_active_organization_key(&active, Some(&pending.previous_active.kid))
        .await
}

async fn mint_material(
    ctx: &RotateContext,
    provider_type: SigningProviderType,
    kid: &str,
    active: &SigningKey,
) -> Result<(String, Option<String>, String), DomainError> {
    match provider_type {
        SigningProviderType::PemFile => {
            let organization_id = active.organization_id.ok_or_else(|| {
                DomainError::AuthorizationError(
                    "organization signing key missing organization_id".into(),
                )
            })?;
            mint_pem_material(&ctx.pem_root, kid, organization_id)
        }
        SigningProviderType::OpenBaoTransit => {
            let org = active
                .organization_id
                .ok_or_else(|| DomainError::InvalidToken)?;
            iam_domain::entity::signing_key::require_org_transit_binding(
                org,
                &active.provider_key_ref,
                active.credential_ref.as_deref(),
            )?;
            let transit = ctx.transit.as_ref().ok_or_else(|| {
                DomainError::external_service_error(
                    "openbao_transit",
                    "Transit URL not configured — refuse closed",
                )
            })?;
            mint_transit_material(
                transit,
                &format!("org-{org}-{kid}"),
                active.credential_ref.as_deref(),
            )
            .await
        }
        SigningProviderType::AwsKms
        | SigningProviderType::GcpKms
        | SigningProviderType::AzureKeyVault
        | SigningProviderType::RemoteHttp => Err(DomainError::ProviderNotSupported(String::from(
            &provider_type,
        ))),
    }
}

fn mint_pem_material(
    pem_root: &Path,
    kid: &str,
    organization_id: Uuid,
) -> Result<(String, Option<String>, String), DomainError> {
    let org_dir = pem_root.join(organization_id.to_string());
    std::fs::create_dir_all(&org_dir).map_err(|e| {
        DomainError::external_service_error(
            "organization_signer_rotate",
            &format!("pem_root create failed: {e}"),
        )
    })?;
    let mut rng = rand::thread_rng();
    let private = RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|e| DomainError::AuthorizationError(format!("RSA 2048 keygen failed: {e}")))?;
    let private_pem = private
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| DomainError::AuthorizationError(format!("PKCS8 encode failed: {e}")))?;
    let public_pem = private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| DomainError::AuthorizationError(format!("SPKI encode failed: {e}")))?;

    let file_name = format!("{kid}.pem");
    let path = org_dir.join(&file_name);
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(DomainError::AuthorizationError(
            "PEM path must not contain '..'".into(),
        ));
    }
    std::fs::write(&path, private_pem.as_bytes()).map_err(|e| {
        DomainError::external_service_error(
            "organization_signer_rotate",
            &format!("write private PEM failed: {e}"),
        )
    })?;
    let provider_key_ref = format!("{organization_id}/{kid}.pem");
    Ok((provider_key_ref, None, public_pem))
}

async fn mint_transit_material(
    transit: &TransitClientConfig,
    kid: &str,
    credential_ref: Option<&str>,
) -> Result<(String, Option<String>, String), DomainError> {
    let token_ref = credential_ref
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            DomainError::AuthorizationError("organization Transit credential required".into())
        })?;
    let provider = TransitSigningProvider::new(
        transit.base_url.clone(),
        kid,
        token_ref,
        transit.workload.clone(),
        None,
    )?;
    provider.create_rsa2048_key().await?;
    let public_key = provider.public_key().await?;
    Ok((kid.to_string(), Some(token_ref.to_string()), public_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::{PemSigningProvider, FORBIDDEN_TRANSIT_KEY_NAME};
    use crate::token::JwtTokenService;
    use iam_domain::entity::token::JwkSet;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRegistry {
        keys: Mutex<Vec<SigningKey>>,
    }

    #[async_trait]
    impl SigningKeyRegistry for FakeRegistry {
        type Error = DomainError;
        async fn jwks_publication_snapshot(
            &self,
        ) -> Result<iam_domain::entity::signing_key::SigningKeyPublicationSnapshot, Self::Error>
        {
            Ok(
                iam_domain::entity::signing_key::SigningKeyPublicationSnapshot {
                    keys: self.keys.lock().unwrap().clone(),
                    as_of: Utc::now(),
                },
            )
        }

        async fn confirm_active_for_emission(
            &self,
            expected: &SigningKey,
        ) -> Result<bool, Self::Error> {
            // Compare the expected object, not merely kid/id; timestamps are not epochs.
            Ok(expected.status == SigningKeyStatus::Active
                && self.keys.lock().unwrap().iter().any(|row| {
                    let mut row = row.clone();
                    row.created_at = expected.created_at;
                    row.updated_at = expected.updated_at;
                    row == *expected
                }))
        }

        async fn insert(&self, key: &SigningKey) -> Result<(), Self::Error> {
            self.keys.lock().unwrap().push(key.clone());
            Ok(())
        }

        async fn replace_active_organization_key(
            &self,
            key: &SigningKey,
            expected_active_kid: Option<&str>,
        ) -> Result<SigningKey, Self::Error> {
            let mut keys = self.keys.lock().unwrap();
            if let Some(expected) = expected_active_kid {
                if !keys
                    .iter()
                    .any(|row| row.kid == expected && row.status == SigningKeyStatus::Active)
                {
                    return Err(DomainError::InvalidToken);
                }
            }
            for row in keys.iter_mut().filter(|row| {
                row.organization_id == key.organization_id && row.status == SigningKeyStatus::Active
            }) {
                row.status = SigningKeyStatus::Retiring;
                row.updated_at = key.updated_at;
            }
            if let Some(row) = keys.iter_mut().find(|row| row.id == key.id) {
                *row = key.clone();
            } else {
                keys.push(key.clone());
            }
            Ok(key.clone())
        }

        async fn revoke_organization_keys(
            &self,
            organization_id: Uuid,
        ) -> Result<Vec<SigningKey>, Self::Error> {
            let mut keys = self.keys.lock().unwrap();
            let mut revoked = Vec::new();
            for row in keys
                .iter_mut()
                .filter(|row| row.organization_id == Some(organization_id))
            {
                row.status = SigningKeyStatus::Revoked;
                row.updated_at = Utc::now();
                revoked.push(row.clone());
            }
            Ok(revoked)
        }

        async fn find_by_kid(&self, kid: &str) -> Result<Option<SigningKey>, Self::Error> {
            Ok(self
                .keys
                .lock()
                .unwrap()
                .iter()
                .find(|k| k.kid == kid)
                .cloned())
        }

        async fn find_active_platform_key(&self) -> Result<Option<SigningKey>, Self::Error> {
            Ok(None)
        }

        async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, Self::Error> {
            Ok(self
                .keys
                .lock()
                .unwrap()
                .iter()
                .filter(|k| k.status.in_jwks())
                .cloned()
                .collect())
        }

        async fn update(&self, key: &SigningKey) -> Result<(), Self::Error> {
            let mut keys = self.keys.lock().unwrap();
            if let Some(slot) = keys.iter_mut().find(|k| k.id == key.id) {
                *slot = key.clone();
            }
            Ok(())
        }

        async fn find_by_organization(
            &self,
            organization_id: Uuid,
        ) -> Result<Vec<SigningKey>, Self::Error> {
            Ok(self
                .keys
                .lock()
                .unwrap()
                .iter()
                .filter(|k| k.organization_id == Some(organization_id))
                .cloned()
                .collect())
        }

        async fn find_by_issuer(&self, issuer: &str) -> Result<Vec<SigningKey>, Self::Error> {
            Ok(self
                .keys
                .lock()
                .unwrap()
                .iter()
                .filter(|k| k.issuer == issuer)
                .cloned()
                .collect())
        }
    }

    fn sample_active(org_id: Uuid, public_key: &str) -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: opaque_kid(),
            algorithm: "RS256".to_string(),
            trust_scope: TrustScope::Organization,
            issuer: "http://127.0.0.1/iam/orgs/acme".to_string(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "seed.pem".to_string(),
            credential_ref: None,
            public_key: public_key.to_string(),
            status: SigningKeyStatus::Active,
            organization_id: Some(org_id),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn pending_before_promote_is_in_jwks_but_cannot_sign() {
        let org_id = Uuid::new_v4();
        let public = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let active = sample_active(org_id, public);
        registry.insert(&active).await.unwrap();

        let pem_root = std::env::temp_dir().join(format!("aiforall-rotate-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&pem_root).unwrap();
        let ctx = RotateContext {
            registry: registry.clone(),
            pem_root,
            transit: None,
        };

        let pending = insert_pending_rotation(&ctx, &active)
            .await
            .expect("insert pending");
        assert_eq!(pending.pending.status, SigningKeyStatus::Pending);
        assert!(!pending.pending.status.can_sign());
        assert_ne!(pending.pending.public_key, active.public_key);
        assert!(!pending.pending.kid.starts_with("org-"));
        assert!(
            pending
                .pending
                .provider_key_ref
                .starts_with(&format!("{org_id}/")),
            "org PEM ref must be stored under {{org_id}}/"
        );

        let jwks = JwkSet::from_registry_keys(&registry.list_jwks_keys().await.unwrap());
        assert!(jwks.keys.iter().any(|k| k.kid == pending.pending.kid));

        let encoder =
            JwtTokenService::with_hmac("test-secret-at-least-32-bytes-long!!".into(), 900)
                .with_signing_provider(
                    Arc::new(
                        PemSigningProvider::new(
                            include_str!("../../../config/keys/test-platform.pem"),
                            public,
                        )
                        .expect("pem"),
                    ),
                    "platform-boot-kid",
                    "http://127.0.0.1/iam",
                    JwkSet { keys: vec![] },
                );
        assert_eq!(encoder.signing_kid(), Some("platform-boot-kid"));
        assert_ne!(encoder.signing_kid().unwrap(), pending.pending.kid);

        let promoted = promote_pending_rotation(&ctx, pending)
            .await
            .expect("promote");
        assert_eq!(promoted.status, SigningKeyStatus::Active);
        let keys = registry.find_by_organization(org_id).await.unwrap();
        assert!(keys.iter().any(|k| k.status == SigningKeyStatus::Retiring));
        assert!(keys
            .iter()
            .any(|k| k.status == SigningKeyStatus::Active && k.kid == promoted.kid));
    }

    #[tokio::test]
    async fn full_rotate_returns_active_with_new_public_key() {
        let org_id = Uuid::new_v4();
        let public = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let active = sample_active(org_id, public);
        registry.insert(&active).await.unwrap();
        let pem_root =
            std::env::temp_dir().join(format!("aiforall-rotate-full-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&pem_root).unwrap();
        let ctx = RotateContext {
            registry: registry.clone(),
            pem_root,
            transit: None,
        };
        let new_key = rotate_organization_signer(&ctx, org_id)
            .await
            .expect("rotate");
        assert_eq!(new_key.status, SigningKeyStatus::Active);
        assert_ne!(new_key.public_key, public);
        assert!(!new_key.kid.starts_with("org-"));
        assert!(
            new_key.provider_key_ref.starts_with(&format!("{org_id}/")),
            "org PEM ref must be stored under {{org_id}}/"
        );
    }

    #[tokio::test]
    async fn insert_pending_rotation_rejects_forbidden_transit_key_name() {
        let org_id = Uuid::new_v4();
        let public = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let mut active = sample_active(org_id, public);
        active.provider_type = SigningProviderType::OpenBaoTransit;
        active.provider_key_ref = FORBIDDEN_TRANSIT_KEY_NAME.to_string();
        registry.insert(&active).await.unwrap();
        let pem_root =
            std::env::temp_dir().join(format!("aiforall-rotate-cosign-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&pem_root).unwrap();
        let ctx = RotateContext {
            registry,
            pem_root,
            transit: None,
        };
        let err = insert_pending_rotation(&ctx, &active)
            .await
            .expect_err("forbidden transit key");
        assert!(
            matches!(err, DomainError::AuthorizationError(ref msg) if msg.contains("apparatus-p4-cosign"))
        );
    }

    #[tokio::test]
    async fn rotate_pem_rejects_missing_organization_id() {
        let org_id = Uuid::new_v4();
        let public = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let mut active = sample_active(org_id, public);
        active.organization_id = None;
        let pem_root =
            std::env::temp_dir().join(format!("aiforall-rotate-no-org-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&pem_root).unwrap();
        let ctx = RotateContext {
            registry,
            pem_root,
            transit: None,
        };
        let err = insert_pending_rotation(&ctx, &active)
            .await
            .expect_err("organization_id required");
        assert!(
            matches!(err, DomainError::AuthorizationError(ref msg) if msg.contains("organization_id"))
        );
    }
}
