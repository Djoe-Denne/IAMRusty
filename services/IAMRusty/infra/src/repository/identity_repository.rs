//! `SeaORM` `Identity` repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::identity::{Identity, IdentityKind};
use iam_domain::error::DomainError;
use iam_domain::port::repository::IdentityRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, RuntimeErr,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::repository::entity::identities::{self, Entity as Identities};

/// Postgres-backed identity repository.
#[derive(Clone)]
pub struct SeaOrmIdentityRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmIdentityRepository {
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn insert_identity(&self, identity: &Identity) -> Result<Identity, DbErr> {
        let active = identities::ActiveModel {
            id: ActiveValue::Set(identity.id),
            user_id: ActiveValue::Set(identity.user_id),
            issuer: ActiveValue::Set(identity.issuer.clone()),
            subject: ActiveValue::Set(identity.subject.clone()),
            kind: ActiveValue::Set(String::from(&identity.kind)),
            created_at: ActiveValue::Set(identity.created_at.naive_utc()),
            updated_at: ActiveValue::Set(identity.updated_at.naive_utc()),
        };
        let model = active.insert(self.db.as_ref()).await?;
        to_domain(model).map_err(|error| DbErr::Custom(error.to_string()))
    }
}

fn is_identity_conflict(error: &DbErr) -> bool {
    match error {
        DbErr::Exec(RuntimeErr::SqlxError(sea_orm::sqlx::Error::Database(error)))
        | DbErr::Query(RuntimeErr::SqlxError(sea_orm::sqlx::Error::Database(error))) => {
            error.code().as_deref() == Some("23505")
                && matches!(
                    error.constraint(),
                    Some("idx_identities_issuer_subject" | "idx_identities_user_issuer_kind")
                )
        }
        _ => false,
    }
}

fn to_domain(model: identities::Model) -> Result<Identity, DomainError> {
    Ok(Identity {
        id: model.id,
        user_id: model.user_id,
        issuer: model.issuer,
        subject: model.subject,
        kind: IdentityKind::from_str(&model.kind).map_err(DomainError::RepositoryError)?,
        created_at: DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
        updated_at: DateTime::<Utc>::from_naive_utc_and_offset(model.updated_at, Utc),
    })
}

#[async_trait]
impl IdentityRepository for SeaOrmIdentityRepository {
    type Error = DomainError;

    async fn find_by_issuer_subject(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<Identity>, Self::Error> {
        let model = Identities::find()
            .filter(identities::Column::Issuer.eq(issuer))
            .filter(identities::Column::Subject.eq(subject))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        model.map(to_domain).transpose()
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Identity>, Self::Error> {
        let models = Identities::find()
            .filter(identities::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }

    async fn create(&self, identity: &Identity) -> Result<Identity, Self::Error> {
        self.insert_identity(identity)
            .await
            .map_err(|error| DomainError::RepositoryError(error.to_string()))
    }

    async fn ensure_platform_identity(
        &self,
        user_id: Uuid,
        issuer: &str,
    ) -> Result<Identity, Self::Error> {
        let subject = user_id.to_string();
        if let Some(existing) = self.find_by_issuer_subject(issuer, &subject).await? {
            return Ok(existing);
        }
        // Never return another issuer's Platform row (historical `iamrusty` vs URL).
        let candidate = Identity::platform(user_id, issuer);
        match self.insert_identity(&candidate).await {
            Ok(identity) => Ok(identity),
            Err(error) if is_identity_conflict(&error) => {
                let existing = self
                    .find_by_issuer_subject(issuer, &subject)
                    .await?
                    .ok_or_else(|| {
                        DomainError::RepositoryError("platform identity conflict".into())
                    })?;
                if existing.user_id != user_id || existing.kind != IdentityKind::Platform {
                    return Err(DomainError::RepositoryError(
                        "platform identity conflict".into(),
                    ));
                }
                Ok(existing)
            }
            Err(error) => Err(DomainError::RepositoryError(error.to_string())),
        }
    }

    async fn ensure_organization_managed_identity(
        &self,
        user_id: Uuid,
        issuer: &str,
    ) -> Result<Identity, Self::Error> {
        let existing = self.find_by_user_id(user_id).await?;
        if let Some(found) = existing
            .into_iter()
            .find(|i| i.issuer == issuer && matches!(i.kind, IdentityKind::OrganizationManaged))
        {
            return Ok(found);
        }
        self.create(&Identity::organization_managed(user_id, issuer))
            .await
    }
}
