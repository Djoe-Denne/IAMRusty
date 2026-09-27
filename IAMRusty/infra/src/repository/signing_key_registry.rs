//! SeaORM SigningKeyRegistry implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::signing_key::{
    SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
};
use iam_domain::error::DomainError;
use iam_domain::port::repository::SigningKeyRegistry;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::repository::entity::signing_keys::{self, Entity as SigningKeys};

/// Postgres-backed signing key registry.
#[derive(Clone)]
pub struct SeaOrmSigningKeyRegistry {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmSigningKeyRegistry {
    #[must_use]
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn to_domain(model: signing_keys::Model) -> Result<SigningKey, DomainError> {
    Ok(SigningKey {
        id: model.id,
        kid: model.kid,
        algorithm: model.algorithm,
        trust_scope: match model.trust_scope.as_str() {
            "platform" => TrustScope::Platform,
            "organization" => TrustScope::Organization,
            other => {
                return Err(DomainError::RepositoryError(format!(
                    "unknown trust_scope: {other}"
                )))
            }
        },
        issuer: model.issuer,
        provider_type: SigningProviderType::from_str(&model.provider_type)
            .map_err(DomainError::RepositoryError)?,
        provider_key_ref: model.provider_key_ref,
        credential_ref: model.credential_ref,
        public_key: model.public_key,
        status: SigningKeyStatus::from_str(&model.status).map_err(DomainError::RepositoryError)?,
        organization_id: model.organization_id,
        created_at: DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
        updated_at: DateTime::<Utc>::from_naive_utc_and_offset(model.updated_at, Utc),
    })
}

fn to_active(key: &SigningKey) -> signing_keys::ActiveModel {
    signing_keys::ActiveModel {
        id: ActiveValue::Set(key.id),
        kid: ActiveValue::Set(key.kid.clone()),
        algorithm: ActiveValue::Set(key.algorithm.clone()),
        trust_scope: ActiveValue::Set(match key.trust_scope {
            TrustScope::Platform => "platform".to_string(),
            TrustScope::Organization => "organization".to_string(),
        }),
        issuer: ActiveValue::Set(key.issuer.clone()),
        provider_type: ActiveValue::Set(String::from(&key.provider_type)),
        provider_key_ref: ActiveValue::Set(key.provider_key_ref.clone()),
        credential_ref: ActiveValue::Set(key.credential_ref.clone()),
        public_key: ActiveValue::Set(key.public_key.clone()),
        status: ActiveValue::Set(String::from(&key.status)),
        organization_id: ActiveValue::Set(key.organization_id),
        created_at: ActiveValue::Set(key.created_at.naive_utc()),
        updated_at: ActiveValue::Set(key.updated_at.naive_utc()),
    }
}

#[async_trait]
impl SigningKeyRegistry for SeaOrmSigningKeyRegistry {
    type Error = DomainError;

    async fn insert(&self, key: &SigningKey) -> Result<(), Self::Error> {
        to_active(key)
            .insert(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_kid(&self, kid: &str) -> Result<Option<SigningKey>, Self::Error> {
        let model = SigningKeys::find()
            .filter(signing_keys::Column::Kid.eq(kid))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        model.map(to_domain).transpose()
    }

    async fn find_active_platform_key(&self) -> Result<Option<SigningKey>, Self::Error> {
        let model = SigningKeys::find()
            .filter(signing_keys::Column::TrustScope.eq("platform"))
            .filter(signing_keys::Column::Status.eq("active"))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        model.map(to_domain).transpose()
    }

    async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, Self::Error> {
        let models = SigningKeys::find()
            .filter(signing_keys::Column::Status.is_in(["pending", "active", "retiring"]))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }

    async fn update(&self, key: &SigningKey) -> Result<(), Self::Error> {
        to_active(key)
            .update(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<SigningKey>, Self::Error> {
        let models = SigningKeys::find()
            .filter(signing_keys::Column::OrganizationId.eq(organization_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }

    async fn find_by_issuer(&self, issuer: &str) -> Result<Vec<SigningKey>, Self::Error> {
        let models = SigningKeys::find()
            .filter(signing_keys::Column::Issuer.eq(issuer))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }
}

/// Bootstrap an active platform key when none exists.
///
/// When `provider_type` is [`SigningProviderType::RemoteHttp`] and an active platform key
/// already exists (e.g. prior PEM bootstrap), upserts kid / public key / provider to the
/// remote material so JWKS never stays on a PEM overlay while HSM signs.
///
/// # Errors
///
/// Returns [`DomainError`] on DB or insert/update failure.
pub async fn bootstrap_platform_signing_key(
    registry: &SeaOrmSigningKeyRegistry,
    kid: &str,
    issuer: &str,
    public_key_pem: &str,
    provider_key_ref: &str,
    provider_type: SigningProviderType,
) -> Result<SigningKey, DomainError> {
    if let Some(mut existing) = registry.find_active_platform_key().await? {
        if provider_type == SigningProviderType::RemoteHttp {
            let needs_upsert = existing.provider_type != SigningProviderType::RemoteHttp
                || existing.kid != kid
                || existing.public_key != public_key_pem
                || existing.provider_key_ref != provider_key_ref
                || existing.issuer != issuer;
            if needs_upsert {
                existing.kid = kid.to_string();
                existing.issuer = issuer.to_string();
                existing.provider_type = SigningProviderType::RemoteHttp;
                existing.provider_key_ref = provider_key_ref.to_string();
                existing.public_key = public_key_pem.to_string();
                existing.updated_at = Utc::now();
                registry.update(&existing).await?;
            }
            return Ok(existing);
        }
        return Ok(existing);
    }
    let now = Utc::now();
    let key = SigningKey {
        id: Uuid::new_v4(),
        kid: kid.to_string(),
        algorithm: "RS256".to_string(),
        trust_scope: TrustScope::Platform,
        issuer: issuer.to_string(),
        provider_type,
        provider_key_ref: provider_key_ref.to_string(),
        credential_ref: None,
        public_key: public_key_pem.to_string(),
        status: SigningKeyStatus::Active,
        organization_id: None,
        created_at: now,
        updated_at: now,
    };
    registry.insert(&key).await?;
    Ok(key)
}
