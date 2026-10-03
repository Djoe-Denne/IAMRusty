//! Organization signer internal RPC (ADR-0306) — guarded by `x-iam-internal-token` only.

use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    Extension, Json,
};
use iam_application::usecase::organization_signer::{
    ConfigureOrganizationSignerInput, OrganizationSignerFacade, OrganizationSignerFacadeImpl,
    OrganizationSignerResult, ISSUER_OWNED_BY_OTHER_ORGANIZATION,
    NO_ACTIVE_ORGANIZATION_SIGNING_KEY,
};
use iam_domain::error::DomainError;
use iam_domain::port::repository::{IdentityRepository, SigningKeyRegistry};
use iam_domain::port::{OrganizationSignerProbe, OrganizationSignerRotator};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::rate_limit::require_internal_service_token;

/// Runtime context for org-signer / org-identity internal routes.
#[derive(Clone)]
pub struct SignerRouteContext {
    pub facade: Arc<dyn OrganizationSignerFacade>,
    pub registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
    pub identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
    pub public_base_url: String,
    pub probe: Arc<dyn OrganizationSignerProbe>,
    pub rotator: Arc<dyn OrganizationSignerRotator>,
    pub pem_root: PathBuf,
    pub expiration_seconds: u64,
    /// Clock skew seconds for retiring JWKS retention (typically 60).
    pub skew_seconds: u64,
    /// Optional Transit base URL (mint / probe). Never taken from Hive body.
    pub transit_base_url: Option<String>,
}

impl SignerRouteContext {
    /// Build context and wire the application façade from the same ports.
    #[must_use]
    pub fn new(
        registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
        identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
        public_base_url: impl Into<String>,
        probe: Arc<dyn OrganizationSignerProbe>,
        rotator: Arc<dyn OrganizationSignerRotator>,
        pem_root: PathBuf,
        expiration_seconds: u64,
        skew_seconds: u64,
        transit_base_url: Option<String>,
    ) -> Self {
        let public_base_url = public_base_url.into();
        let facade = Arc::new(OrganizationSignerFacadeImpl::new(
            registry.clone(),
            public_base_url.clone(),
            probe.clone(),
            rotator.clone(),
        )) as Arc<dyn OrganizationSignerFacade>;
        Self {
            facade,
            registry,
            identity_repo,
            public_base_url,
            probe,
            rotator,
            pem_root,
            expiration_seconds,
            skew_seconds,
            transit_base_url,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConfigureSignerBody {
    pub provider_type: String,
    pub provider_key_ref: String,
    #[serde(default)]
    pub credential_ref: Option<String>,
    pub public_key: String,
    /// Trusted from Hive (internal token). Hive binds this to `Organization.slug`.
    pub org_slug: String,
}

#[derive(Debug, Serialize)]
pub struct SignerResponse {
    pub signing_profile_id: Uuid,
    pub kid: String,
    pub status: String,
    pub issuer: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationIdentityBody {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct OrganizationIdentityResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub issuer: String,
    pub subject: String,
    pub kind: String,
}

fn map_signer_error(err: DomainError) -> StatusCode {
    match err {
        DomainError::BusinessRuleViolation(ref message)
            if message == ISSUER_OWNED_BY_OTHER_ORGANIZATION =>
        {
            StatusCode::CONFLICT
        }
        DomainError::AuthorizationError(ref message)
            if message.contains(NO_ACTIVE_ORGANIZATION_SIGNING_KEY) =>
        {
            StatusCode::NOT_FOUND
        }
        DomainError::AuthorizationError(_)
        | DomainError::ProviderNotSupported(_)
        | DomainError::BusinessRuleViolation(_)
        | DomainError::TokenValidationFailed(_)
        | DomainError::InvalidToken => StatusCode::BAD_REQUEST,
        DomainError::ExternalServiceError { .. } => StatusCode::BAD_GATEWAY,
        DomainError::UserNotFound | DomainError::TokenNotFound => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn map_rotate_error(err: DomainError) -> StatusCode {
    map_signer_error(err)
}

fn map_probe_error(err: DomainError) -> StatusCode {
    match err {
        DomainError::AuthorizationError(_)
        | DomainError::TokenValidationFailed(_)
        | DomainError::ProviderNotSupported(_)
        | DomainError::BusinessRuleViolation(_) => StatusCode::BAD_REQUEST,
        DomainError::ExternalServiceError { .. } => StatusCode::BAD_GATEWAY,
        _ => StatusCode::BAD_GATEWAY,
    }
}

impl From<OrganizationSignerResult> for SignerResponse {
    fn from(value: OrganizationSignerResult) -> Self {
        Self {
            signing_profile_id: value.signing_profile_id,
            kid: value.kid,
            status: value.status,
            issuer: value.issuer,
        }
    }
}

/// POST `/internal/organizations/{org_id}/signer/configure`
///
/// `org_slug` is trusted from Hive (internal token). Hive binds it to `Organization.slug`.
pub async fn configure_organization_signer(
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Extension(ctx): Extension<Arc<SignerRouteContext>>,
    Json(body): Json<ConfigureSignerBody>,
) -> Result<Json<SignerResponse>, StatusCode> {
    require_internal_service_token(&headers)?;
    let result = ctx
        .facade
        .configure(
            org_id,
            &ConfigureOrganizationSignerInput {
                provider_type: body.provider_type,
                provider_key_ref: body.provider_key_ref,
                credential_ref: body.credential_ref,
                public_key: body.public_key,
                org_slug: body.org_slug,
            },
        )
        .await
        .map_err(map_signer_error)?;
    Ok(Json(result.into()))
}

/// POST `/internal/organizations/{org_id}/signer/test`
pub async fn test_organization_signer(
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Extension(ctx): Extension<Arc<SignerRouteContext>>,
) -> Result<Json<SignerResponse>, StatusCode> {
    require_internal_service_token(&headers)?;
    let result = ctx.facade.test(org_id).await.map_err(|e| {
        // Preserve probe BAD_GATEWAY vs BAD_REQUEST distinction for challenge failures.
        match &e {
            DomainError::AuthorizationError(m)
                if m.contains(NO_ACTIVE_ORGANIZATION_SIGNING_KEY) =>
            {
                StatusCode::NOT_FOUND
            }
            DomainError::AuthorizationError(_)
            | DomainError::TokenValidationFailed(_)
            | DomainError::ProviderNotSupported(_)
            | DomainError::BusinessRuleViolation(_) => map_probe_error(e),
            DomainError::ExternalServiceError { .. } => map_probe_error(e),
            _ => map_signer_error(e),
        }
    })?;
    Ok(Json(result.into()))
}

/// POST `/internal/organizations/{org_id}/signer/rotate`
pub async fn rotate_organization_signer(
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Extension(ctx): Extension<Arc<SignerRouteContext>>,
) -> Result<Json<SignerResponse>, StatusCode> {
    require_internal_service_token(&headers)?;
    let result = ctx.facade.rotate(org_id).await.map_err(map_rotate_error)?;
    Ok(Json(result.into()))
}

/// POST `/internal/organizations/{org_id}/signer/disable`
pub async fn disable_organization_signer(
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Extension(ctx): Extension<Arc<SignerRouteContext>>,
) -> Result<Json<SignerResponse>, StatusCode> {
    require_internal_service_token(&headers)?;
    let result = ctx.facade.disable(org_id).await.map_err(map_signer_error)?;
    Ok(Json(result.into()))
}

/// POST `/internal/organizations/{org_id}/identities` — ensure org-managed identity.
pub async fn create_organization_managed_identity(
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Extension(ctx): Extension<Arc<SignerRouteContext>>,
    Json(body): Json<CreateOrganizationIdentityBody>,
) -> Result<Json<OrganizationIdentityResponse>, StatusCode> {
    require_internal_service_token(&headers)?;
    let keys = ctx
        .registry
        .find_by_organization(org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let active = keys
        .into_iter()
        .find(|k| k.status.can_sign())
        .ok_or(StatusCode::NOT_FOUND)?;
    let identity = ctx
        .identity_repo
        .ensure_organization_managed_identity(body.user_id, &active.issuer)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OrganizationIdentityResponse {
        id: identity.id,
        user_id: identity.user_id,
        issuer: identity.issuer,
        subject: identity.subject,
        kind: String::from(&identity.kind),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use iam_application::usecase::organization_signer::require_org_scoped_pem_ref;
    use iam_domain::entity::identity::{Identity, IdentityKind};
    use iam_domain::entity::signing_key::{
        opaque_kid, SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
        FORBIDDEN_TRANSIT_KEY_NAME,
    };
    use iam_domain::entity::token::JwkSet;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn sample_org_key(org_id: Uuid, issuer: &str, public_key: &str) -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: opaque_kid(),
            algorithm: "RS256".to_string(),
            trust_scope: TrustScope::Organization,
            issuer: issuer.to_string(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "opaque".to_string(),
            credential_ref: None,
            public_key: public_key.to_string(),
            status: SigningKeyStatus::Active,
            organization_id: Some(org_id),
            created_at: now,
            updated_at: now,
        }
    }

    fn test_signer_ctx(
        registry: Arc<FakeRegistry>,
        identity_repo: Arc<FakeIdentityRepo>,
        rotator: Arc<dyn OrganizationSignerRotator>,
    ) -> Arc<SignerRouteContext> {
        Arc::new(SignerRouteContext::new(
            registry,
            identity_repo,
            "http://127.0.0.1",
            Arc::new(NoopProbe),
            rotator,
            std::env::temp_dir(),
            900,
            60,
            None,
        ))
    }

    #[derive(Default)]
    struct FakeRegistry {
        keys: Mutex<Vec<SigningKey>>,
    }

    #[async_trait]
    impl SigningKeyRegistry for FakeRegistry {
        type Error = DomainError;

        async fn insert(&self, key: &SigningKey) -> Result<(), Self::Error> {
            self.keys.lock().unwrap().push(key.clone());
            Ok(())
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

    #[derive(Default)]
    struct FakeIdentityRepo {
        rows: Mutex<HashMap<(Uuid, String, String), Identity>>,
    }

    #[async_trait]
    impl IdentityRepository for FakeIdentityRepo {
        type Error = DomainError;

        async fn find_by_issuer_subject(
            &self,
            _issuer: &str,
            _subject: &str,
        ) -> Result<Option<Identity>, Self::Error> {
            Ok(None)
        }

        async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Identity>, Self::Error> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .values()
                .filter(|i| i.user_id == user_id)
                .cloned()
                .collect())
        }

        async fn create(&self, identity: &Identity) -> Result<Identity, Self::Error> {
            self.rows.lock().unwrap().insert(
                (
                    identity.user_id,
                    identity.issuer.clone(),
                    String::from(&identity.kind),
                ),
                identity.clone(),
            );
            Ok(identity.clone())
        }

        async fn ensure_platform_identity(
            &self,
            user_id: Uuid,
            issuer: &str,
        ) -> Result<Identity, Self::Error> {
            self.create(&Identity::platform(user_id, issuer)).await
        }

        async fn ensure_organization_managed_identity(
            &self,
            user_id: Uuid,
            issuer: &str,
        ) -> Result<Identity, Self::Error> {
            let key = (
                user_id,
                issuer.to_string(),
                String::from(&IdentityKind::OrganizationManaged),
            );
            if let Some(existing) = self.rows.lock().unwrap().get(&key).cloned() {
                return Ok(existing);
            }
            self.create(&Identity::organization_managed(user_id, issuer))
                .await
        }
    }

    struct NoopProbe;
    #[async_trait]
    impl OrganizationSignerProbe for NoopProbe {
        async fn challenge(&self, _key: &SigningKey) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct StubRotator {
        registry: Arc<FakeRegistry>,
        next_public_key: String,
    }

    #[async_trait]
    impl OrganizationSignerRotator for StubRotator {
        async fn rotate(&self, organization_id: Uuid) -> Result<SigningKey, DomainError> {
            let keys = self.registry.find_by_organization(organization_id).await?;
            let mut active = keys
                .into_iter()
                .find(|k| k.status.can_sign())
                .ok_or_else(|| DomainError::AuthorizationError("missing".into()))?;
            active.status = SigningKeyStatus::Retiring;
            active.updated_at = Utc::now();
            self.registry.update(&active).await?;

            let now = Utc::now();
            let new_key = SigningKey {
                id: Uuid::new_v4(),
                kid: opaque_kid(),
                algorithm: "RS256".to_string(),
                trust_scope: TrustScope::Organization,
                issuer: active.issuer.clone(),
                provider_type: SigningProviderType::PemFile,
                provider_key_ref: "rotated.pem".to_string(),
                credential_ref: None,
                public_key: self.next_public_key.clone(),
                status: SigningKeyStatus::Active,
                organization_id: Some(organization_id),
                created_at: now,
                updated_at: now,
            };
            self.registry.insert(&new_key).await?;
            Ok(new_key)
        }
    }

    fn internal_headers() -> HeaderMap {
        crate::configure_internal_service_token("iam-handler-unit-token");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-iam-internal-token",
            "iam-handler-unit-token".parse().expect("header"),
        );
        headers
    }

    #[tokio::test]
    async fn reject_invalid_rsa_public_pem_via_configure() {
        let org_id = Uuid::new_v4();
        let registry = Arc::new(FakeRegistry::default());
        let ctx = test_signer_ctx(
            registry,
            Arc::new(FakeIdentityRepo::default()),
            Arc::new(StubRotator {
                registry: Arc::new(FakeRegistry::default()),
                next_public_key: "x".into(),
            }),
        );
        let err = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(ConfigureSignerBody {
                provider_type: "pem_file".into(),
                provider_key_ref: format!("{org_id}/kid.pem"),
                credential_ref: None,
                public_key: "not-a-pem".into(),
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("bad pem");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn reject_oversized_rsa_public_pem_via_configure() {
        let org_id = Uuid::new_v4();
        let registry = Arc::new(FakeRegistry::default());
        let ctx = test_signer_ctx(
            registry,
            Arc::new(FakeIdentityRepo::default()),
            Arc::new(StubRotator {
                registry: Arc::new(FakeRegistry::default()),
                next_public_key: "x".into(),
            }),
        );
        let oversized = "x".repeat(16 * 1024 + 1);
        let err = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(ConfigureSignerBody {
                provider_type: "pem_file".into(),
                provider_key_ref: format!("{org_id}/kid.pem"),
                credential_ref: None,
                public_key: oversized,
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("oversized pem");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn configure_kid_is_opaque_without_org_prefix() {
        let kid = opaque_kid();
        assert!(!kid.starts_with("org-"));
        assert_eq!(kid.len(), 32);
    }

    #[tokio::test]
    async fn rotate_http_promotes_active_and_retires_previous() {
        let org_id = Uuid::new_v4();
        let old_pub = include_str!("../../../config/keys/test-platform.pub");
        let new_pub = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu1SU1LfVLPHCozMxH2Mo\n4lgOEePzNm0tRgeLezV6ffAt0gunVTLw7onLRnrq0/IzW7yWR7QkrmBL7jTKEn5u\n+qKhbwKfBstIs+bMY2Zkp18gnTxKLxoS2tFczGkPLPgizskuemMghRniWaoLcyeh\nkd3qqGElvW/VDL5AaWTg0nLVkjRo9z+40RQzuVaE8AkAFmxZzow3x+VJYKdjykkJ\n0iT9wCS0DRZSjGtGhQxZHXcEOTIKK9EHi4OUVzy/F+bs/bTW9sZ/L/YRt1DVnGGt\nY5k4EDEbOQPd1cq61Yx0ZQIDAQAB\n-----END PUBLIC KEY-----\n";
        let registry = Arc::new(FakeRegistry::default());
        let active = sample_org_key(org_id, "http://127.0.0.1/iam/orgs/acme", old_pub);
        registry.insert(&active).await.unwrap();

        let rotator = Arc::new(StubRotator {
            registry: registry.clone(),
            next_public_key: new_pub.to_string(),
        });
        let ctx = test_signer_ctx(
            registry.clone(),
            Arc::new(FakeIdentityRepo::default()),
            rotator,
        );

        let response = rotate_organization_signer(internal_headers(), Path(org_id), Extension(ctx))
            .await
            .expect("rotate 200");
        assert_eq!(response.0.status, "active");
        assert_ne!(response.0.kid, active.kid);

        let keys = registry.find_by_organization(org_id).await.unwrap();
        assert!(keys.iter().any(|k| k.status == SigningKeyStatus::Retiring));
        let new_active = keys
            .iter()
            .find(|k| k.status == SigningKeyStatus::Active)
            .expect("active");
        assert_ne!(new_active.public_key, old_pub);
    }

    #[tokio::test]
    async fn create_org_identity_requires_active_key() {
        let org_id = Uuid::new_v4();
        let registry = Arc::new(FakeRegistry::default());
        let ctx = test_signer_ctx(
            registry.clone(),
            Arc::new(FakeIdentityRepo::default()),
            Arc::new(StubRotator {
                registry,
                next_public_key: "x".into(),
            }),
        );
        let err = create_organization_managed_identity(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(CreateOrganizationIdentityBody {
                user_id: Uuid::new_v4(),
            }),
        )
        .await
        .expect_err("no active key");
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn pending_key_appears_in_jwks_before_promote() {
        let org_id = Uuid::new_v4();
        let registry = Arc::new(FakeRegistry::default());
        let active_pub = include_str!("../../../config/keys/test-platform.pub");
        let active = sample_org_key(org_id, "http://127.0.0.1/iam/orgs/acme", active_pub);
        registry.insert(&active).await.unwrap();

        let now = Utc::now();
        let pending = SigningKey {
            id: Uuid::new_v4(),
            kid: opaque_kid(),
            algorithm: "RS256".to_string(),
            trust_scope: TrustScope::Organization,
            issuer: active.issuer.clone(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "n1.pem".into(),
            credential_ref: None,
            public_key: active_pub.to_string(),
            status: SigningKeyStatus::Pending,
            organization_id: Some(org_id),
            created_at: now,
            updated_at: now,
        };
        registry.insert(&pending).await.unwrap();

        let jwks = JwkSet::from_registry_keys(&registry.list_jwks_keys().await.unwrap());
        assert!(jwks.keys.iter().any(|k| k.kid == pending.kid));
        assert!(!pending.status.can_sign());
        let platform_kid = "platform-boot-kid";
        assert_ne!(platform_kid, pending.kid);
    }

    #[tokio::test]
    async fn create_org_identity_idempotent_when_active_key_exists() {
        let org_id = Uuid::new_v4();
        let registry = Arc::new(FakeRegistry::default());
        let pub_pem = include_str!("../../../config/keys/test-platform.pub");
        let active = sample_org_key(org_id, "http://127.0.0.1/iam/orgs/acme", pub_pem);
        registry.insert(&active).await.unwrap();
        let identities = Arc::new(FakeIdentityRepo::default());
        let ctx = test_signer_ctx(
            registry.clone(),
            identities.clone(),
            Arc::new(StubRotator {
                registry,
                next_public_key: "x".into(),
            }),
        );
        let user = Uuid::new_v4();
        let first = create_organization_managed_identity(
            internal_headers(),
            Path(org_id),
            Extension(ctx.clone()),
            Json(CreateOrganizationIdentityBody { user_id: user }),
        )
        .await
        .expect("create");
        let second = create_organization_managed_identity(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(CreateOrganizationIdentityBody { user_id: user }),
        )
        .await
        .expect("idempotent");
        assert_eq!(first.0.id, second.0.id);
        assert_ne!(first.0.subject, user.to_string());
        assert_eq!(first.0.kind, "organization_managed");
    }

    #[test]
    fn map_rotate_error_404_only_for_missing_active_key() {
        assert_eq!(
            map_rotate_error(DomainError::AuthorizationError(
                "no active organization signing key".into()
            )),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            map_rotate_error(DomainError::AuthorizationError(
                "Transit key name apparatus-p4-cosign is forbidden".into()
            )),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            map_rotate_error(DomainError::AuthorizationError(
                "PEM path must not contain '..'".into()
            )),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            map_rotate_error(DomainError::AuthorizationError(
                "RSA 2048 keygen failed: entropy".into()
            )),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn configure_pem_ref_must_stay_under_org_subtree() {
        let org_id = Uuid::new_v4();
        let other = Uuid::new_v4();
        assert!(require_org_scoped_pem_ref(org_id, "test-platform.pem").is_err());
        assert!(require_org_scoped_pem_ref(org_id, &format!("{other}/kid.pem")).is_err());
        assert!(require_org_scoped_pem_ref(org_id, "../test-platform.pem").is_err());
        assert!(require_org_scoped_pem_ref(org_id, &format!("{org_id}/kid.pem")).is_ok());
    }

    #[tokio::test]
    async fn configure_pem_rejects_platform_and_foreign_org_refs() {
        let org_id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let pub_pem = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let ctx = test_signer_ctx(
            registry.clone(),
            Arc::new(FakeIdentityRepo::default()),
            Arc::new(StubRotator {
                registry,
                next_public_key: "x".into(),
            }),
        );

        let platform = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx.clone()),
            Json(ConfigureSignerBody {
                provider_type: "pem_file".into(),
                provider_key_ref: "test-platform.pem".into(),
                credential_ref: None,
                public_key: pub_pem.to_string(),
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("platform pem");
        assert_eq!(platform, StatusCode::BAD_REQUEST);

        let foreign = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx.clone()),
            Json(ConfigureSignerBody {
                provider_type: "pem_file".into(),
                provider_key_ref: format!("{other}/kid.pem"),
                credential_ref: None,
                public_key: pub_pem.to_string(),
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("foreign org pem");
        assert_eq!(foreign, StatusCode::BAD_REQUEST);

        let traversal = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(ConfigureSignerBody {
                provider_type: "pem_file".into(),
                provider_key_ref: "../test-platform.pem".into(),
                credential_ref: None,
                public_key: pub_pem.to_string(),
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("dotdot");
        assert_eq!(traversal, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn configure_pem_rejects_org_prefixed_traversal() {
        let org_id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let pub_pem = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let ctx = test_signer_ctx(
            registry.clone(),
            Arc::new(FakeIdentityRepo::default()),
            Arc::new(StubRotator {
                registry: registry.clone(),
                next_public_key: "x".into(),
            }),
        );

        let err = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(ConfigureSignerBody {
                provider_type: "pem_file".into(),
                provider_key_ref: format!("{org_id}/../{other}/kid.pem"),
                credential_ref: None,
                public_key: pub_pem.to_string(),
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("org-prefixed traversal");
        assert_eq!(err, StatusCode::BAD_REQUEST);
        assert!(registry.keys.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn configure_transit_rejects_forbidden_cosign_key() {
        let org_id = Uuid::new_v4();
        let pub_pem = include_str!("../../../config/keys/test-platform.pub");
        let registry = Arc::new(FakeRegistry::default());
        let ctx = test_signer_ctx(
            registry.clone(),
            Arc::new(FakeIdentityRepo::default()),
            Arc::new(StubRotator {
                registry,
                next_public_key: "x".into(),
            }),
        );

        let err = configure_organization_signer(
            internal_headers(),
            Path(org_id),
            Extension(ctx),
            Json(ConfigureSignerBody {
                provider_type: "openbao_transit".into(),
                provider_key_ref: FORBIDDEN_TRANSIT_KEY_NAME.to_string(),
                credential_ref: None,
                public_key: pub_pem.to_string(),
                org_slug: "acme".into(),
            }),
        )
        .await
        .expect_err("cosign key name");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }
}
