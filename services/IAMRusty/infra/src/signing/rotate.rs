//! Provider-owned platform / organization rotation N+1 (ADR-0304 / ADR-0306).

use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::signing_key::{
    opaque_kid, same_effective_signing_binding, PreparedSigningTransition, SigningKey,
    SigningKeyPreparation, SigningKeyStatus, SigningProviderType, SigningScope,
};
use iam_domain::error::DomainError;
use iam_domain::port::repository::SigningKeyRegistry;
use iam_domain::port::{
    OrganizationSignerProbe, OrganizationSignerRotator, SigningProvider, WorkloadIdentity,
};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use super::transit::{TransitSigningProvider, FORBIDDEN_TRANSIT_KEY_NAME};

/// Optional `OpenBao` Transit client used for org-key mint / probe.
#[derive(Clone)]
pub struct TransitClientConfig {
    pub base_url: String,
    pub workload: Arc<dyn WorkloadIdentity>,
    pub token_ref: String,
}

/// Dependencies for provider rotation and promotion in either signing scope.
#[derive(Clone)]
pub struct RotateContext {
    pub registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
    pub pem_root: PathBuf,
    pub transit: Option<TransitClientConfig>,
    /// Explicit trusted LocalInsecure/IsolatedTest choice, never inferred from URL.
    pub allow_local_pem: bool,
}

/// Intermediate state after Pending insert and before Active promote (testable seam).
#[derive(Debug, Clone)]
pub struct PendingRotation {
    pub pending: SigningKey,
    pub previous_active: SigningKey,
    pub prepared: PreparedSigningTransition,
}

/// Default rotator wired from setup into HTTP `SignerRouteContext`.
pub struct DefaultOrganizationSignerRotator {
    ctx: RotateContext,
}

impl DefaultOrganizationSignerRotator {
    #[must_use]
    pub const fn new(ctx: RotateContext) -> Self {
        Self { ctx }
    }

    #[must_use]
    pub const fn context(&self) -> &RotateContext {
        &self.ctx
    }
}

#[async_trait]
impl OrganizationSignerRotator for DefaultOrganizationSignerRotator {
    async fn rotate_scope(&self, scope: &SigningScope) -> Result<SigningKey, DomainError> {
        rotate_signing_scope(&self.ctx, scope).await
    }
    async fn rotate(&self, organization_id: Uuid) -> Result<SigningKey, DomainError> {
        rotate_organization_signer(&self.ctx, organization_id).await
    }
}

/// Full rotate: provider successor → Pending → N+1 Active / retire previous.
///
/// # Errors
///
/// Returns [`DomainError`] when no Active key exists or material / persistence fails.
pub async fn rotate_organization_signer(
    ctx: &RotateContext,
    organization_id: Uuid,
) -> Result<SigningKey, DomainError> {
    rotate_signing_scope(ctx, &SigningScope::organization(organization_id)).await
}

/// # Errors
///
/// Returns [`DomainError`] when no Active key exists or material / persistence fails.
pub async fn rotate_signing_scope(
    ctx: &RotateContext,
    scope: &SigningScope,
) -> Result<SigningKey, DomainError> {
    let before = ctx.registry.signing_scope_snapshot(scope).await?;
    let active = before.active.ok_or_else(|| {
        DomainError::AuthorizationError("no active organization signing key".into())
    })?;
    let pending = insert_pending_rotation(ctx, &active).await?;
    promote_pending_rotation(ctx, pending).await
}

/// Obtain a provider-owned successor and insert Pending (visible, non-signing).
///
/// # Errors
///
/// Returns [`DomainError`] on unsupported provider / Transit / registry failure.
pub async fn insert_pending_rotation(
    ctx: &RotateContext,
    active: &SigningKey,
) -> Result<PendingRotation, DomainError> {
    if active.provider_type == SigningProviderType::PemFile && !ctx.allow_local_pem {
        return Err(DomainError::InvalidSigningKeyMaterial);
    }
    if active.trust_scope == iam_domain::entity::signing_key::TrustScope::Organization
        && active.organization_id.is_none()
    {
        return Err(DomainError::AuthorizationError(
            "organization signing key missing organization_id".into(),
        ));
    }
    SigningScope::of(active).validate()?;
    // Local fixtures may Sign existing material, never mint a new private key.
    if active.provider_type != SigningProviderType::OpenBaoTransit {
        return Err(DomainError::ProviderNotSupported(String::from(
            &active.provider_type,
        )));
    }
    let before = ctx
        .registry
        .signing_scope_snapshot(&SigningScope::of(active))
        .await?;
    let current = before.active.as_ref().ok_or(DomainError::InvalidToken)?;
    if current.id != active.id
        || current.kid != active.kid
        || !same_effective_signing_binding(current, active)
    {
        return Err(iam_domain::entity::signing_key::admission_denied(
            iam_domain::entity::signing_key::SigningKeyAdmissionReason::EpochConflict,
        ));
    }
    if let Some(prepared) = before.pending.as_ref() {
        if prepared.previous_active_kid.as_deref() != Some(active.kid.as_str()) {
            return Err(DomainError::InvalidToken);
        }
        return Ok(PendingRotation {
            pending: prepared.key.clone(),
            previous_active: active.clone(),
            prepared: prepared.clone(),
        });
    }
    let kid = opaque_kid();
    if kid == FORBIDDEN_TRANSIT_KEY_NAME || active.provider_key_ref == FORBIDDEN_TRANSIT_KEY_NAME {
        return Err(DomainError::AuthorizationError(
            "Transit key name apparatus-p4-cosign is forbidden".into(),
        ));
    }

    let (provider_key_ref, provider_key_version, credential_ref, public_key) =
        rotate_provider_material(ctx, active).await?;

    let now = Utc::now();
    let pending = SigningKey {
        id: Uuid::new_v4(),
        kid,
        algorithm: "RS256".to_string(),
        trust_scope: active.trust_scope.clone(),
        issuer: active.issuer.clone(),
        provider_type: active.provider_type.clone(),
        provider_key_ref,
        provider_key_version,
        credential_ref,
        public_key,
        status: SigningKeyStatus::Pending,
        organization_id: active.organization_id,
        created_at: now,
        updated_at: now,
    };
    // Version-pinned public and proof of possession, before any writer lock.
    let mut probe = super::probe::DefaultOrganizationSignerProbe::new(ctx.pem_root.clone())
        .with_local_pem_allowed(ctx.allow_local_pem);
    if let Some(transit) = &ctx.transit {
        probe = probe.with_transit(transit.clone());
    }
    let proof = probe.prove(pending, before).await?;
    let SigningKeyPreparation::Pending(prepared) = ctx.registry.prepare_signing_key(proof).await?
    else {
        return Err(DomainError::InvalidToken);
    };
    Ok(PendingRotation {
        pending: prepared.key.clone(),
        previous_active: active.clone(),
        prepared,
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
    if pending.pending != pending.prepared.key
        || pending.prepared.previous_active_kid.as_deref()
            != Some(pending.previous_active.kid.as_str())
    {
        return Err(DomainError::InvalidToken);
    }
    ctx.registry.promote_signing_key(&pending.prepared).await
}

async fn rotate_provider_material(
    ctx: &RotateContext,
    active: &SigningKey,
) -> Result<(String, Option<u32>, Option<String>, String), DomainError> {
    match &active.provider_type {
        SigningProviderType::PemFile => {
            if !ctx.allow_local_pem {
                return Err(DomainError::InvalidSigningKeyMaterial);
            }
            Err(DomainError::ProviderNotSupported(String::from(
                &active.provider_type,
            )))
        }
        SigningProviderType::OpenBaoTransit => {
            SigningScope::of(active).validate()?;
            if let Some(org) = active.organization_id {
                iam_domain::entity::signing_key::require_org_transit_binding(
                    org,
                    &active.provider_key_ref,
                    active.credential_ref.as_deref(),
                )?;
            } else {
                iam_domain::entity::signing_key::require_transit_key_name(
                    &active.provider_key_ref,
                )?;
            }
            let transit = ctx.transit.as_ref().ok_or_else(|| {
                DomainError::external_service_error(
                    "openbao_transit",
                    "Transit URL not configured — refuse closed",
                )
            })?;
            if active.organization_id.is_none()
                && active.credential_ref.as_deref() != Some(transit.token_ref.as_str())
            {
                return Err(DomainError::InvalidSigningKeyMaterial);
            }
            mint_transit_material(
                transit,
                &active.provider_key_ref,
                iam_domain::entity::signing_key::require_transit_key_version(
                    active.provider_key_version,
                )?,
                &active.public_key,
                active.credential_ref.as_deref(),
            )
            .await
        }
        SigningProviderType::AwsKms
        | SigningProviderType::GcpKms
        | SigningProviderType::AzureKeyVault
        | SigningProviderType::RemoteHttp => Err(DomainError::ProviderNotSupported(String::from(
            &active.provider_type,
        ))),
    }
}

async fn mint_transit_material(
    transit: &TransitClientConfig,
    key_name: &str,
    active_version: u32,
    active_public: &str,
    credential_ref: Option<&str>,
) -> Result<(String, Option<u32>, Option<String>, String), DomainError> {
    let token_ref = credential_ref
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            DomainError::AuthorizationError("organization Transit credential required".into())
        })?;
    let provider = TransitSigningProvider::new(
        transit.base_url.clone(),
        key_name,
        active_version,
        token_ref,
        transit.workload.clone(),
        Some(active_public.to_string()),
    )?;
    provider.public_key().await?;
    let version = provider.rotate_rsa2048_key().await?;
    let pinned = TransitSigningProvider::new(
        transit.base_url.clone(),
        key_name,
        version,
        token_ref,
        transit.workload.clone(),
        None,
    )?;
    let public_key = pinned.public_key().await?;
    Ok((
        key_name.to_string(),
        Some(version),
        Some(token_ref.to_string()),
        public_key,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::{PemSigningProvider, FORBIDDEN_TRANSIT_KEY_NAME};
    use crate::token::JwtTokenService;
    use iam_domain::entity::signing_key::TrustScope;
    use iam_domain::entity::token::JwkSet;
    use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::RsaPrivateKey;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRegistry {
        keys: Mutex<Vec<SigningKey>>,
        epochs: Mutex<std::collections::HashMap<Option<Uuid>, u64>>,
        prepared: Mutex<Vec<PreparedSigningTransition>>,
    }

    #[async_trait]
    impl SigningKeyRegistry for FakeRegistry {
        type Error = DomainError;
        async fn signing_scope_snapshot(
            &self,
            scope: &SigningScope,
        ) -> Result<iam_domain::entity::signing_key::SigningScopeSnapshot, DomainError> {
            let snapshot = {
                let keys = self.keys.lock().unwrap();
                let epochs = self.epochs.lock().unwrap();
                let prepared = self.prepared.lock().unwrap();
                iam_domain::entity::signing_key::SigningScopeSnapshot {
                    scope: scope.clone(),
                    revision: *epochs.get(&scope.organization_id).unwrap_or(&0),
                    active: keys
                        .iter()
                        .find(|key| {
                            SigningScope::of(key) == *scope
                                && key.status == SigningKeyStatus::Active
                        })
                        .cloned(),
                    pending: prepared
                        .iter()
                        .find(|p| SigningScope::of(&p.key) == *scope)
                        .cloned(),
                }
            };
            Ok(snapshot)
        }
        async fn prepare_signing_key(
            &self,
            proof: iam_domain::entity::signing_key::ProbedSigningKey,
        ) -> Result<SigningKeyPreparation, DomainError> {
            let mut keys = self.keys.lock().unwrap();
            let mut epochs = self.epochs.lock().unwrap();
            let mut prepared = self.prepared.lock().unwrap();
            let key = proof.key();
            let outcome = {
                let revision = epochs.entry(key.organization_id).or_default();
                if *revision != proof.before().revision {
                    Err(DomainError::InvalidToken)
                } else if let Some(existing) = prepared
                    .iter()
                    .find(|p| SigningScope::of(&p.key) == SigningScope::of(key))
                {
                    if !same_effective_signing_binding(&existing.key, key) {
                        Err(DomainError::InvalidToken)
                    } else {
                        Ok(SigningKeyPreparation::Pending(existing.clone()))
                    }
                } else {
                    *revision += 1;
                    let pending = PreparedSigningTransition {
                        key: key.clone(),
                        revision: *revision,
                        previous_active_kid: proof
                            .before()
                            .active
                            .as_ref()
                            .map(|key| key.kid.clone()),
                    };
                    keys.push(key.clone());
                    prepared.push(pending.clone());
                    Ok(SigningKeyPreparation::Pending(pending))
                }
            };
            drop((keys, epochs, prepared));
            outcome
        }
        async fn promote_signing_key(
            &self,
            expected: &PreparedSigningTransition,
        ) -> Result<SigningKey, DomainError> {
            let mut keys = self.keys.lock().unwrap();
            let mut epochs = self.epochs.lock().unwrap();
            let mut prepared = self.prepared.lock().unwrap();
            let revision = epochs.entry(expected.key.organization_id).or_default();
            if *revision != expected.revision
                || !prepared
                    .iter()
                    .any(|p| p.key == expected.key && p.revision == expected.revision)
            {
                return Err(DomainError::InvalidToken);
            }
            let now = Utc::now();
            for key in keys.iter_mut().filter(|key| {
                SigningScope::of(key) == SigningScope::of(&expected.key)
                    && key.status == SigningKeyStatus::Active
            }) {
                key.status = SigningKeyStatus::Retiring;
                key.updated_at = now;
            }
            let key = keys
                .iter_mut()
                .find(|key| key.id == expected.key.id && key.status == SigningKeyStatus::Pending)
                .ok_or(DomainError::InvalidToken)?;
            key.status = SigningKeyStatus::Active;
            key.updated_at = now;
            let key = key.clone();
            prepared.retain(|p| p.key.id != key.id);
            *revision += 1;
            Ok(key)
        }
        async fn jwks_publication_snapshot(
            &self,
        ) -> Result<iam_domain::entity::signing_key::SigningKeyPublicationSnapshot, Self::Error>
        {
            iam_domain::entity::signing_key::SigningKeyPublicationSnapshot::from_fixture_keys(
                self.keys.lock().unwrap().clone(),
                Utc::now(),
                900,
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
            *self
                .epochs
                .lock()
                .unwrap()
                .entry(key.organization_id)
                .or_default() += 1;
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
            let replaced = key.clone();
            drop(keys);
            Ok(replaced)
        }

        async fn revoke_organization_keys(
            &self,
            organization_id: Uuid,
        ) -> Result<Vec<SigningKey>, Self::Error> {
            let mut keys = self.keys.lock().unwrap();
            let mut revoked = Vec::new();
            *self
                .epochs
                .lock()
                .unwrap()
                .entry(Some(organization_id))
                .or_default() += 1;
            self.prepared
                .lock()
                .unwrap()
                .retain(|p| p.key.organization_id != Some(organization_id));
            for row in keys
                .iter_mut()
                .filter(|row| row.organization_id == Some(organization_id))
            {
                row.status = SigningKeyStatus::Revoked;
                row.updated_at = Utc::now();
                revoked.push(row.clone());
            }
            drop(keys);
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
            Ok(self
                .keys
                .lock()
                .unwrap()
                .iter()
                .find(|key| {
                    key.organization_id.is_none()
                        && key.trust_scope == TrustScope::Platform
                        && key.status == SigningKeyStatus::Active
                })
                .cloned())
        }

        async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, Self::Error> {
            let listed = {
                let keys = self.keys.lock().unwrap();
                keys.iter()
                    .filter(|k| k.status.in_jwks())
                    .cloned()
                    .collect()
            };
            Ok(listed)
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
            provider_key_version: None,
            credential_ref: None,
            public_key: public_key.to_string(),
            status: SigningKeyStatus::Active,
            organization_id: Some(org_id),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn rotation_refuses_local_pem_without_opt_in_before_registry_or_filesystem_io() {
        let org = Uuid::new_v4();
        let active = sample_active(org, include_str!("../../../config/keys/test-platform.pub"));
        let registry = Arc::new(FakeRegistry::default());
        registry.insert(&active).await.unwrap();
        // Deliberately do not create this path. Denial must precede keygen/write.
        let root = std::env::temp_dir().join(format!("iam-denied-rotate-{}", Uuid::new_v4()));
        let ctx = RotateContext {
            registry: registry.clone(),
            pem_root: root.clone(),
            transit: None,
            allow_local_pem: false,
        };
        assert!(matches!(
            insert_pending_rotation(&ctx, &active).await,
            Err(DomainError::InvalidSigningKeyMaterial)
        ));
        assert!(matches!(
            rotate_provider_material(&ctx, &active).await,
            Err(DomainError::InvalidSigningKeyMaterial)
        ));
        assert!(!root.exists());
        assert_eq!(*registry.keys.lock().unwrap(), vec![active]);
        assert!(registry.prepared.lock().unwrap().is_empty());
    }

    /// Isolated outbound fixture with genuine distinct RSA materials for v7/v8.
    /// The private test keys stay in memory in the fake vendor, never IAM files.
    async fn transit_rotation_fixture(
        scope: SigningScope,
        registry: Arc<FakeRegistry>,
    ) -> (
        rustycog::testing::wiremock::MockServerFixture,
        RotateContext,
        SigningKey,
    ) {
        use base64::Engine;
        use rsa::pkcs8::DecodePrivateKey;
        use sha2::Sha256;
        use std::sync::atomic::{AtomicBool, Ordering};
        use wiremock::{
            matchers::{method, path},
            Mock, ResponseTemplate,
        };
        let fixture = rustycog::testing::wiremock::MockServerFixture::isolated().await;
        let server = fixture.server();
        let owner = scope.organization_id.unwrap_or_else(Uuid::new_v4);
        let key_name = scope.organization_id.map_or_else(
            || format!("platform-{owner}-fixture-key"),
            |org| format!("org-{org}-fixture-key"),
        );
        let credential = scope.organization_id.map_or_else(
            || "platform-fixture-credential".into(),
            |org| format!("org-{org}-fixture-credential"),
        );
        let old_private =
            RsaPrivateKey::from_pkcs8_pem(include_str!("../../../config/keys/test-platform.pem"))
                .unwrap();
        // Crypto succession is the object of this test; not ordinary boot keygen.
        let next_private = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
        let next_public = next_private
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        let public = include_str!("../../../config/keys/test-platform.pub");
        assert_ne!(
            iam_domain::entity::signing_key::parse_signing_public_key(public).unwrap(),
            next_private.to_public_key()
        );
        let rotated = Arc::new(AtomicBool::new(false));
        let for_get = rotated.clone();
        Mock::given(method("GET"))
            .and(path(format!("/v1/transit/keys/{key_name}")))
            .respond_with(move |_: &wiremock::Request| {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                    "type":"rsa-2048","exportable":false,"supports_signing":true,
                    "latest_version":if for_get.load(Ordering::SeqCst) {8} else {7},
                    "keys":{"7":{"public_key":public},"8":{"public_key":next_public}}
                }}))
            })
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v1/transit/keys/{key_name}/rotate")))
            .respond_with(move |_: &wiremock::Request| {
                assert!(
                    !rotated.swap(true, Ordering::SeqCst),
                    "must not retry provider Rotate"
                );
                ResponseTemplate::new(204)
            })
            .mount(&server)
            .await;
        Mock::given(method("POST")).and(path(format!("/v1/transit/sign/{key_name}")))
            .respond_with(move |request: &wiremock::Request| {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let version = body["key_version"].as_u64().unwrap();
                assert_eq!(body["prehashed"], true);
                let digest = base64::engine::general_purpose::STANDARD.decode(body["input"].as_str().unwrap()).unwrap();
                let private = match version {7=>&old_private,8=>&next_private,_=>panic!("unregistered pin")};
                let signature = private.sign(rsa::pkcs1v15::Pkcs1v15Sign::new::<Sha256>(), &digest).unwrap();
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                    "signature":format!("vault:v{version}:{}",base64::engine::general_purpose::STANDARD.encode(signature))
                }}))
            }).mount(&server).await;
        let mut active = sample_active(owner, public);
        active.trust_scope = scope.trust_scope;
        active.organization_id = scope.organization_id;
        if active.organization_id.is_none() {
            active.issuer = "http://127.0.0.1/iam".into();
        }
        active.provider_type = SigningProviderType::OpenBaoTransit;
        active.provider_key_ref = key_name;
        active.provider_key_version = Some(7);
        active.credential_ref = Some(credential.clone());
        registry.insert(&active).await.unwrap();
        let ctx = RotateContext {
            registry,
            // Not created; any accidental filesystem fallback cannot be hidden.
            pem_root: std::env::temp_dir()
                .join(format!("iam-transit-no-private-{}", Uuid::new_v4())),
            transit: Some(TransitClientConfig {
                base_url: server.uri(),
                token_ref: credential.clone(),
                workload: Arc::new(
                    crate::signing::StaticCredential::from_pair(&credential, "fixture-token")
                        .unwrap(),
                ),
            }),
            allow_local_pem: false,
        };
        (fixture, ctx, active)
    }

    #[tokio::test]
    async fn transit_rotation_pins_distinct_successor_and_proves_it_before_pending_insert() {
        use base64::Engine;
        use sha2::{Digest, Sha256};
        use std::sync::atomic::{AtomicBool, Ordering};
        use wiremock::{
            matchers::{body_partial_json, method, path},
            Mock, ResponseTemplate,
        };
        let next_private = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
        let next_public = next_private
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        let digest = Sha256::digest(b"aiforall-org-signer-challenge");
        let signature = next_private
            .sign(rsa::pkcs1v15::Pkcs1v15Sign::new::<Sha256>(), &digest)
            .unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(signature);
        for returned_version in [8, 9] {
            let fixture = rustycog::testing::wiremock::MockServerFixture::isolated().await;
            let server = fixture.server();
            let org = Uuid::new_v4();
            let key_name = format!("org-{org}-fixture-key");
            let credential = format!("org-{org}-fixture-credential");
            let public = include_str!("../../../config/keys/test-platform.pub");
            let rotated = Arc::new(AtomicBool::new(false));
            let for_get = rotated.clone();
            let successor = next_public.clone();
            Mock::given(method("GET")).and(path(format!("/v1/transit/keys/{key_name}")))
                .respond_with(move |_: &wiremock::Request| {
                    let latest = if for_get.load(Ordering::SeqCst) { 8 } else { 7 };
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                        "type":"rsa-2048","exportable":false,"supports_signing":true,
                        "latest_version":latest,"keys":{"7":{"public_key":public},"8":{"public_key":successor}}
                    }}))
                }).mount(&server).await;
            Mock::given(method("POST"))
                .and(path(format!("/v1/transit/keys/{key_name}/rotate")))
                .respond_with(move |_: &wiremock::Request| {
                    rotated.store(true, Ordering::SeqCst);
                    ResponseTemplate::new(204)
                })
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path(format!("/v1/transit/sign/{key_name}")))
                .and(body_partial_json(serde_json::json!({"key_version":8})))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data":{"signature":format!("vault:v{returned_version}:{b64}")}
                })))
                .mount(&server)
                .await;
            let registry = Arc::new(FakeRegistry::default());
            let mut active = sample_active(org, public);
            active.provider_type = SigningProviderType::OpenBaoTransit;
            active.provider_key_ref = key_name.clone();
            active.provider_key_version = Some(7);
            active.credential_ref = Some(credential.clone());
            registry.insert(&active).await.unwrap();
            let root = tempfile::tempdir().unwrap();
            let ctx = RotateContext {
                registry: registry.clone(),
                pem_root: root.path().into(),
                transit: Some(TransitClientConfig {
                    base_url: server.uri(),
                    token_ref: credential.clone(),
                    workload: Arc::new(
                        crate::signing::StaticCredential::from_pair(&credential, "fixture-token")
                            .unwrap(),
                    ),
                }),
                allow_local_pem: false,
            };
            let result = insert_pending_rotation(&ctx, &active).await;
            if returned_version != 8 {
                assert!(
                    result.is_err(),
                    "valid signature in a wrong version envelope is not a PoP"
                );
                assert_eq!(*registry.keys.lock().unwrap(), vec![active]);
                continue;
            }
            let pending = result.unwrap();
            assert_eq!(pending.pending.provider_key_ref, key_name);
            assert_eq!(pending.pending.provider_key_version, Some(8));
            assert_ne!(pending.pending.public_key, active.public_key);
            assert_ne!(pending.pending.kid, active.kid);
            assert!(!pending.pending.status.can_sign());
            let published = registry.list_jwks_keys().await.unwrap();
            assert_eq!(published.len(), 2);
            assert_eq!(
                published
                    .iter()
                    .find(|k| k.kid == active.kid)
                    .unwrap()
                    .status,
                SigningKeyStatus::Active
            );
            let requests_before_resume = server.received_requests().await.unwrap().len();
            let resumed = insert_pending_rotation(&ctx, &active).await.unwrap();
            assert_eq!(resumed.pending, pending.pending);
            assert_eq!(resumed.prepared.revision, pending.prepared.revision);
            assert_eq!(
                server.received_requests().await.unwrap().len(),
                requests_before_resume,
                "durable Pending resume must not Rotate or probe again"
            );
            assert_eq!(registry.list_jwks_keys().await.unwrap().len(), 2);
            let promoted = promote_pending_rotation(&ctx, pending).await.unwrap();
            assert_eq!(promoted.provider_key_version, Some(8));
            let overlap = registry.list_jwks_keys().await.unwrap();
            assert_eq!(overlap.len(), 2);
            assert_eq!(
                overlap.iter().find(|k| k.kid == active.kid).unwrap().status,
                SigningKeyStatus::Retiring
            );
            let json = serde_json::to_value(JwkSet::from_registry_keys_checked(&overlap).unwrap())
                .unwrap();
            assert_eq!(json["keys"].as_array().unwrap().len(), 2);
            for entry in json["keys"].as_array().unwrap() {
                assert!(entry.get("provider_key_ref").is_none());
                assert!(entry.get("provider_key_version").is_none());
            }
            // Both distinct versions remain cryptographically usable offline
            // during overlap; rotating Transit does not revoke an old signature.
            let old_private = RsaPrivateKey::from_pkcs8_pem(include_str!(
                "../../../config/keys/test-platform.pem"
            ))
            .unwrap();
            let old_signature = old_private
                .sign(rsa::pkcs1v15::Pkcs1v15Sign::new::<Sha256>(), &digest)
                .unwrap();
            for (key, signature) in [
                (active.clone(), old_signature),
                (
                    promoted,
                    base64::engine::general_purpose::STANDARD
                        .decode(&b64)
                        .unwrap(),
                ),
            ] {
                let public =
                    iam_domain::entity::signing_key::parse_signing_public_key(&key.public_key)
                        .unwrap();
                public
                    .verify(
                        rsa::pkcs1v15::Pkcs1v15Sign::new::<Sha256>(),
                        &digest,
                        &signature,
                    )
                    .unwrap();
            }
        }
    }

    #[tokio::test]
    async fn pending_before_promote_is_in_jwks_but_cannot_sign() {
        let org_id = Uuid::new_v4();
        let public = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let (_fixture, ctx, active) =
            transit_rotation_fixture(SigningScope::organization(org_id), registry.clone()).await;

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
                .starts_with(&format!("org-{org_id}-")),
            "org Transit ref must belong to the organization namespace"
        );
        assert_eq!(pending.pending.provider_key_ref, active.provider_key_ref);
        assert_eq!(pending.pending.provider_key_version, Some(8));
        assert_eq!(pending.pending.credential_ref, active.credential_ref);
        assert!(
            !ctx.pem_root.exists(),
            "no private files for a provider successor"
        );

        let jwks = JwkSet::from_registry_keys(&registry.list_jwks_keys().await.unwrap());
        assert!(jwks.keys.iter().any(|k| k.kid == pending.pending.kid));

        let encoder =
            JwtTokenService::with_hmac("test-secret-at-least-32-bytes-long!!".into(), 900)
                .with_local_pem_allowed(true) // explicit platform-only fixture, not rotate authority
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
        let (_fixture, ctx, active) =
            transit_rotation_fixture(SigningScope::organization(org_id), registry).await;
        let new_key = rotate_organization_signer(&ctx, org_id)
            .await
            .expect("rotate");
        assert_eq!(new_key.status, SigningKeyStatus::Active);
        assert_ne!(new_key.public_key, public);
        assert!(!new_key.kid.starts_with("org-"));
        assert!(
            new_key
                .provider_key_ref
                .starts_with(&format!("org-{org_id}-")),
            "org Transit ref must belong to the organization namespace"
        );
        assert_eq!(new_key.provider_key_ref, active.provider_key_ref);
        assert_eq!(new_key.provider_key_version, Some(8));
        assert_eq!(new_key.organization_id, Some(org_id));
        assert!(!ctx.pem_root.exists());
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
        let ctx = RotateContext {
            registry,
            pem_root,
            transit: None,
            allow_local_pem: true,
        };
        let err = insert_pending_rotation(&ctx, &active)
            .await
            .expect_err("forbidden transit key");
        assert!(
            matches!(err, DomainError::AuthorizationError(ref msg) if msg.contains("apparatus-p4-cosign"))
        );
    }

    #[tokio::test]
    async fn platform_rotation_uses_the_same_pinned_provider_lifecycle_without_touching_org() {
        let registry = Arc::new(FakeRegistry::default());
        let org = sample_active(
            Uuid::new_v4(),
            include_str!("../../../config/keys/test-platform.pub"),
        );
        registry.insert(&org).await.unwrap();
        let (_fixture, ctx, active) =
            transit_rotation_fixture(SigningScope::platform(), registry.clone()).await;
        let pending = insert_pending_rotation(&ctx, &active).await.unwrap();
        assert_eq!(pending.pending.trust_scope, TrustScope::Platform);
        assert_eq!(pending.pending.organization_id, None);
        assert_eq!(pending.pending.provider_key_ref, active.provider_key_ref);
        assert_eq!(pending.pending.credential_ref, active.credential_ref);
        assert_eq!(pending.pending.provider_key_version, Some(8));
        assert_ne!(pending.pending.public_key, active.public_key);
        assert!(!pending.pending.status.can_sign());
        let published = registry.list_jwks_keys().await.unwrap();
        assert_eq!(published.len(), 3);
        assert_eq!(
            registry.find_active_platform_key().await.unwrap(),
            Some(active.clone())
        );
        let promoted = promote_pending_rotation(&ctx, pending).await.unwrap();
        assert_eq!(
            registry.find_active_platform_key().await.unwrap(),
            Some(promoted.clone())
        );
        assert_eq!(promoted.status, SigningKeyStatus::Active);
        assert_ne!(promoted.kid, active.kid);
        assert_eq!(
            registry
                .find_by_kid(&active.kid)
                .await
                .unwrap()
                .unwrap()
                .status,
            SigningKeyStatus::Retiring
        );
        assert_eq!(registry.find_by_kid(&org.kid).await.unwrap(), Some(org));
        assert!(!ctx.pem_root.exists());
    }

    #[tokio::test]
    async fn even_explicit_local_pem_cannot_rotate_platform_or_org_or_fallback_on_provider_failure()
    {
        for platform in [false, true] {
            let registry = Arc::new(FakeRegistry::default());
            let mut key = sample_active(
                Uuid::new_v4(),
                include_str!("../../../config/keys/test-platform.pub"),
            );
            if platform {
                key.organization_id = None;
                key.trust_scope = TrustScope::Platform;
            }
            registry.insert(&key).await.unwrap();
            let ctx = RotateContext {
                registry: registry.clone(),
                pem_root: std::env::temp_dir()
                    .join(format!("iam-no-local-mint-{}", Uuid::new_v4())),
                transit: None,
                allow_local_pem: true,
            };
            assert!(matches!(
                insert_pending_rotation(&ctx, &key).await,
                Err(DomainError::ProviderNotSupported(_))
            ));
            assert_eq!(*registry.keys.lock().unwrap(), vec![key.clone()]);
            assert!(registry.prepared.lock().unwrap().is_empty());
            assert!(!ctx.pem_root.exists());
            key.provider_type = SigningProviderType::OpenBaoTransit;
            key.provider_key_ref = key.organization_id.map_or_else(
                || "platform-fixture".into(),
                |org| format!("org-{org}-fixture"),
            );
            key.credential_ref = Some(key.organization_id.map_or_else(
                || "platform-credential".into(),
                |org| format!("org-{org}-credential"),
            ));
            key.provider_key_version = Some(7);
            *registry.keys.lock().unwrap() = vec![key.clone()];
            // A locally permitted fixture mode is never a fallback policy.
            assert!(matches!(
                insert_pending_rotation(&ctx, &key).await,
                Err(DomainError::ExternalServiceError { .. })
            ));
            assert_eq!(*registry.keys.lock().unwrap(), vec![key]);
            assert!(registry.prepared.lock().unwrap().is_empty());
            assert!(!ctx.pem_root.exists());
        }
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
        let ctx = RotateContext {
            registry,
            pem_root,
            transit: None,
            allow_local_pem: true,
        };
        let err = insert_pending_rotation(&ctx, &active)
            .await
            .expect_err("organization_id required");
        assert!(
            matches!(err, DomainError::AuthorizationError(ref msg) if msg.contains("organization_id"))
        );
    }
}
