use super::entity::{user_emails, users};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::user::User as DomainUser;
use iam_domain::error::DomainError;
use iam_domain::port::repository::UserWriteRepository;
use iam_domain::service::{
    user_service::{OptionalField, ProfilePatch},
    UserProfile,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, RuntimeErr, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

pub(super) fn is_username_conflict(error: &DbErr) -> bool {
    match error {
        DbErr::Exec(RuntimeErr::SqlxError(sea_orm::sqlx::Error::Database(error)))
        | DbErr::Query(RuntimeErr::SqlxError(sea_orm::sqlx::Error::Database(error))) => {
            error.code().as_deref() == Some("23505")
                && error.constraint() == Some("users_username_unique")
        }
        _ => false,
    }
}

fn profile_storage_error(_: DbErr) -> DomainError {
    DomainError::RepositoryError("profile writer transaction failed".into())
}

/// `SeaORM` implementation of `UserWriteRepository`
#[derive(Clone)]
pub struct UserWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl UserWriteRepositoryImpl {
    /// Create a new `UserWriteRepositoryImpl`
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a domain user to a database model
    fn to_active_model(user: &DomainUser) -> users::ActiveModel {
        users::ActiveModel {
            id: ActiveValue::Set(user.id),
            username: ActiveValue::Set(user.username.clone()),
            password_hash: ActiveValue::Set(user.password_hash.clone()),
            avatar_url: ActiveValue::Set(user.avatar_url.clone()),
            created_at: ActiveValue::Set(user.created_at.naive_utc()),
            updated_at: ActiveValue::Set(user.updated_at.naive_utc()),
        }
    }

    /// Convert a database model to a domain user
    fn to_domain(model: users::Model) -> DomainUser {
        DomainUser {
            id: model.id,
            username: model.username,
            password_hash: model.password_hash,
            avatar_url: model.avatar_url,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(model.updated_at, Utc),
        }
    }
}

#[async_trait]
impl UserWriteRepository for UserWriteRepositoryImpl {
    type Error = DbErr;

    async fn patch_profile(
        &self,
        id: Uuid,
        patch: ProfilePatch,
    ) -> Result<UserProfile, DomainError> {
        let tx = self.db.begin().await.map_err(profile_storage_error)?;
        let existing = super::authentication_session::lock_user(&tx, id)
            .await
            .map_err(|error| match error {
                DbErr::RecordNotFound(_) => DomainError::UserNotFound,
                other => profile_storage_error(other),
            })?;
        let user = if patch.username.is_none() && matches!(patch.avatar_url, OptionalField::Unset) {
            existing
        } else {
            let mut model = users::ActiveModel::from(existing);
            if let Some(username) = patch.username {
                model.username = Set(Some(username));
            }
            if let OptionalField::Set(avatar_url) = patch.avatar_url {
                model.avatar_url = Set(avatar_url);
            }
            model.updated_at = Set(Utc::now().naive_utc());
            model.update(&tx).await.map_err(|error| {
                if is_username_conflict(&error) {
                    DomainError::UsernameTaken
                } else {
                    profile_storage_error(error)
                }
            })?
        };
        let email = user_emails::Entity::find()
            .filter(user_emails::Column::UserId.eq(id))
            .filter(user_emails::Column::IsPrimary.eq(true))
            .one(&tx)
            .await
            .map_err(profile_storage_error)?
            .map(|row| row.email);
        tx.commit().await.map_err(profile_storage_error)?;
        Ok(UserProfile {
            user: Self::to_domain(user),
            email,
        })
    }

    async fn create(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        debug!("Creating new user with ID: {}", user.id);
        let model = Self::to_active_model(&user);

        let res = model.insert(self.db.as_ref()).await?;

        Ok(Self::to_domain(res))
    }

    async fn update(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        debug!("Updating user with ID: {}", user.id);
        let tx = self.db.begin().await?;
        let existing = super::authentication_session::lock_user(&tx, user.id).await?;
        if user.password_hash.is_some() && user.password_hash != existing.password_hash {
            return Err(DbErr::Custom(
                "password mutations require the authentication session writer".into(),
            ));
        }

        let mut model = users::ActiveModel::from(existing);

        if user.username.is_some() {
            model.username = Set(user.username.clone());
        }
        if user.avatar_url.is_some() {
            model.avatar_url = Set(user.avatar_url.clone());
        }
        // Generic profile updates never mutate credentials or restore a replica snapshot.
        model.updated_at = Set(user.updated_at.naive_utc());

        let updated = model.update(&tx).await?;
        tx.commit().await?;

        Ok(Self::to_domain(updated))
    }
}

#[cfg(test)]
mod profile_tests {
    use super::{users, DomainError, OptionalField, ProfilePatch, UserWriteRepositoryImpl};
    use iam_domain::port::repository::UserWriteRepository;
    use sea_orm::{DbBackend, DbErr, MockDatabase};
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn profile_writer_treats_lock_and_update_failures_as_redacted_storage_errors() {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().naive_utc();
        let user = users::Model {
            id,
            username: Some("reader".into()),
            avatar_url: None,
            password_hash: None,
            created_at: now,
            updated_at: now,
        };
        for lock_succeeds in [false, true] {
            let mock = MockDatabase::new(DbBackend::Postgres);
            let mock = if lock_succeeds {
                mock.append_query_results([vec![user.clone()]])
            } else {
                mock
            };
            let db = Arc::new(
                mock.append_query_errors([DbErr::Custom("private database detail".into())])
                    .into_connection(),
            );
            let writer = UserWriteRepositoryImpl::new(db.clone());
            let result = writer
                .patch_profile(
                    id,
                    ProfilePatch {
                        username: Some("changed".into()),
                        avatar_url: OptionalField::Unset,
                    },
                )
                .await;
            assert!(matches!(result, Err(DomainError::RepositoryError(message))
                if message == "profile writer transaction failed"));
            drop(writer);
            let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
            assert_eq!(transactions.len(), 1);
            let log = format!("{transactions:?}").replace("\\\"", "\"");
            assert!(log.contains("ROLLBACK"));
            assert_eq!(log.contains("UPDATE \"users\""), lock_succeeds);
            assert!(!log.contains("\"user_emails\""));
        }
    }
}
