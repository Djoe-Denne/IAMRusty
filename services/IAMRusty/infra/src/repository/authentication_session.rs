//! Credential mutation and refresh issuance share the writer's actual user-row lock.

use async_trait::async_trait;
use iam_domain::{
    entity::token::{parse_auth_epoch, RefreshToken},
    error::DomainError,
    port::repository::AuthenticationSessionWriter,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use uuid::Uuid;

use super::entity::{password_reset_tokens, refresh_tokens, users};

pub struct SeaOrmAuthenticationSessionWriter {
    writer: Arc<DatabaseConnection>,
}

impl SeaOrmAuthenticationSessionWriter {
    #[must_use]
    pub const fn new(writer: Arc<DatabaseConnection>) -> Self {
        Self { writer }
    }
}

/// Every refresh mutation (issue/rotate/revoke/reset) must take this lock first.
pub(crate) async fn lock_user(
    tx: &DatabaseTransaction,
    user_id: Uuid,
) -> Result<users::Model, DbErr> {
    users::Entity::find_by_id(user_id)
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("session owner not found".into()))
}

fn storage_error(_: DbErr) -> DomainError {
    DomainError::RepositoryError("authentication session transaction failed".into())
}

pub(crate) fn read_epoch(value: &str) -> Result<u64, DbErr> {
    parse_auth_epoch(value)
        .ok_or_else(|| DbErr::Custom("invalid stored authentication epoch".into()))
}

#[async_trait]
impl AuthenticationSessionWriter for SeaOrmAuthenticationSessionWriter {
    async fn complete_registration(
        &self,
        user_id: Uuid,
        expected_password_hash: Option<String>,
        username: String,
        token: Option<RefreshToken>,
    ) -> Result<iam_domain::entity::user::User, DomainError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, user_id).await.map_err(storage_error)?;
        if user.password_hash != expected_password_hash || user.username.is_some() {
            return Err(DomainError::InvalidToken);
        }
        let auth_epoch = read_epoch(&user.auth_epoch).map_err(storage_error)?;
        if token
            .as_ref()
            .is_some_and(|token| token.auth_epoch != auth_epoch)
        {
            return Err(DomainError::InvalidToken);
        }
        let mut updated: users::ActiveModel = user.into();
        updated.username = Set(Some(username));
        updated.updated_at = Set(chrono::Utc::now().naive_utc());
        let updated = updated.update(&tx).await.map_err(|error| {
            if super::user_write::is_username_conflict(&error) {
                DomainError::UsernameTaken
            } else {
                storage_error(error)
            }
        })?;
        if let Some(token) = token {
            if token.user_id != user_id || !token.is_valid || token.expires_at <= chrono::Utc::now()
            {
                return Err(DomainError::InvalidToken);
            }
            refresh_tokens::ActiveModel {
                id: Set(token.id),
                user_id: Set(user_id),
                token: Set(RefreshToken::hash_token(&token.token)),
                is_valid: Set(true),
                created_at: Set(token.created_at.into()),
                expires_at: Set(token.expires_at.into()),
                auth_epoch: Set(auth_epoch.to_string()),
            }
            .insert(&tx)
            .await
            .map_err(storage_error)?;
        }
        tx.commit().await.map_err(storage_error)?;
        Ok(iam_domain::entity::user::User {
            id: updated.id,
            username: updated.username,
            password_hash: updated.password_hash,
            auth_epoch,
            avatar_url: updated.avatar_url,
            created_at: chrono::DateTime::from_naive_utc_and_offset(
                updated.created_at,
                chrono::Utc,
            ),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(
                updated.updated_at,
                chrono::Utc,
            ),
        })
    }

    async fn issue(
        &self,
        token: RefreshToken,
        expected_password_hash: Option<String>,
    ) -> Result<(), DomainError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, token.user_id).await.map_err(storage_error)?;
        // The Argon2 proof was obtained before entering this transaction. A reset
        // winning the lock changes the hash and invalidates that proof.
        if user.password_hash != expected_password_hash
            || read_epoch(&user.auth_epoch).map_err(storage_error)? != token.auth_epoch
            || !token.is_valid
            || token.expires_at <= chrono::Utc::now()
        {
            return Err(DomainError::InvalidToken);
        }
        refresh_tokens::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token: Set(RefreshToken::hash_token(&token.token)),
            is_valid: Set(true),
            created_at: Set(token.created_at.into()),
            expires_at: Set(token.expires_at.into()),
            auth_epoch: Set(token.auth_epoch.to_string()),
        }
        .insert(&tx)
        .await
        .map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)
    }

    async fn change_password(
        &self,
        user_id: Uuid,
        expected_password_hash: Option<String>,
        new_password_hash: String,
    ) -> Result<(), DomainError> {
        self.write_password(user_id, expected_password_hash, new_password_hash, None)
            .await
    }

    async fn recover_password(
        &self,
        user_id: Uuid,
        expected_password_hash: Option<String>,
        new_password_hash: String,
        reset_token_hash: String,
    ) -> Result<(), DomainError> {
        self.write_password(
            user_id,
            expected_password_hash,
            new_password_hash,
            Some(reset_token_hash),
        )
        .await
    }
}

impl SeaOrmAuthenticationSessionWriter {
    async fn write_password(
        &self,
        user_id: Uuid,
        expected_password_hash: Option<String>,
        new_password_hash: String,
        reset_token_hash: Option<String>,
    ) -> Result<(), DomainError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, user_id).await.map_err(storage_error)?;
        if user.password_hash != expected_password_hash {
            return Err(DomainError::InvalidToken);
        }
        let next_epoch = if let Some(hash) = reset_token_hash {
            // Revalidate on the writer inside the same critical section. A stale
            // read-replica token or simultaneous callback cannot win twice.
            let valid = password_reset_tokens::Entity::find()
                .filter(password_reset_tokens::Column::UserId.eq(user_id))
                .filter(password_reset_tokens::Column::TokenHash.eq(hash))
                .filter(password_reset_tokens::Column::UsedAt.is_null())
                .filter(password_reset_tokens::Column::ExpiresAt.gt(chrono::Utc::now()))
                .one(&tx)
                .await
                .map_err(storage_error)?;
            if valid.is_none() {
                return Err(DomainError::InvalidToken);
            }
            Some(
                read_epoch(&user.auth_epoch)
                    .map_err(storage_error)?
                    .checked_add(1)
                    .ok_or_else(|| {
                        DomainError::RepositoryError("authentication epoch exhausted".into())
                    })?,
            )
        } else if expected_password_hash.is_none() {
            return Err(DomainError::InvalidToken);
        } else {
            None
        };
        let mut updated: users::ActiveModel = user.into();
        if let Some(epoch) = next_epoch {
            updated.auth_epoch = Set(epoch.to_string());
        }
        updated.password_hash = Set(Some(new_password_hash));
        updated.updated_at = Set(chrono::Utc::now().naive_utc());
        updated.update(&tx).await.map_err(storage_error)?;
        refresh_tokens::Entity::delete_many()
            .filter(refresh_tokens::Column::UserId.eq(user_id))
            .exec(&tx)
            .await
            .map_err(storage_error)?;
        password_reset_tokens::Entity::delete_many()
            .filter(password_reset_tokens::Column::UserId.eq(user_id))
            .exec(&tx)
            .await
            .map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, MockDatabase};

    fn owner(user_id: Uuid, hash: &str) -> users::Model {
        let now = chrono::Utc::now().naive_utc();
        users::Model {
            id: user_id,
            username: Some("registered".into()),
            avatar_url: None,
            created_at: now,
            updated_at: now,
            password_hash: Some(hash.into()),
            auth_epoch: "0".into(),
        }
    }

    #[tokio::test]
    async fn complete_registration_rejects_stale_epoch_before_any_mutation() {
        let user_id = Uuid::new_v4();
        let mut user = owner(user_id, "current-password-proof");
        user.username = None;
        user.auth_epoch = "1".into();
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![user.clone()], vec![user]])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        let result = writer
            .complete_registration(
                user_id,
                Some("current-password-proof".into()),
                "registered".into(),
                Some(refresh(user_id)),
            )
            .await;
        assert!(matches!(result, Err(DomainError::InvalidToken)));
        drop(writer);
        let log = format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log());
        assert!(log.contains("FOR UPDATE"));
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("INSERT INTO"));
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("COMMIT"));
    }

    #[tokio::test]
    async fn exhausted_recovery_epoch_never_updates_password_or_purges_sessions() {
        let user_id = Uuid::new_v4();
        let mut user = owner(user_id, "old");
        user.auth_epoch = u64::MAX.to_string();
        let now = chrono::Utc::now();
        let proof = password_reset_tokens::Model {
            id: Uuid::new_v4(),
            user_id,
            token_hash: "valid-proof".into(),
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            used_at: None,
        };
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![user]])
                .append_query_results([vec![proof]])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        assert!(matches!(
            writer
                .recover_password(user_id, Some("old".into()), "new".into(), "valid-proof".into())
                .await,
            Err(DomainError::RepositoryError(message)) if message == "authentication epoch exhausted"
        ));
        drop(writer);
        let log = format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log());
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("DELETE FROM"));
        assert!(!log.contains("COMMIT"));
    }

    fn refresh(user_id: Uuid) -> RefreshToken {
        let now = chrono::Utc::now();
        RefreshToken {
            auth_epoch: 0,
            id: Uuid::new_v4(),
            user_id,
            token: "unit-refresh-secret".into(),
            is_valid: true,
            created_at: now,
            expires_at: now + chrono::Duration::hours(1),
        }
    }

    #[tokio::test]
    async fn registration_username_write_failure_is_not_reported_as_a_collision() {
        let id = Uuid::new_v4();
        let mut applicant = owner(id, "current-hash");
        applicant.username = None;
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![applicant]])
                .append_query_errors([DbErr::Custom("private username update error".into())])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        let result = writer
            .complete_registration(
                id,
                Some("current-hash".into()),
                "requested".into(),
                Some(refresh(id)),
            )
            .await;
        assert!(matches!(result, Err(DomainError::RepositoryError(message))
            if message == "authentication session transaction failed"));
        drop(writer);
        let log = format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log());
        assert!(log.contains("UPDATE"));
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("INSERT INTO"));
    }

    #[tokio::test]
    async fn complete_registration_stale_hash_or_existing_username_never_mutates() {
        for existing_username in [false, true] {
            let id = Uuid::new_v4();
            let mut current = owner(id, "current-hash");
            if !existing_username {
                current.username = None;
            }
            let db = Arc::new(
                MockDatabase::new(DbBackend::Postgres)
                    .append_query_results([vec![current]])
                    .into_connection(),
            );
            let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
            let proof = if existing_username {
                "current-hash"
            } else {
                "stale-hash"
            };
            assert!(matches!(
                writer
                    .complete_registration(
                        id,
                        Some(proof.into()),
                        "requested".into(),
                        Some(refresh(id))
                    )
                    .await,
                Err(DomainError::InvalidToken)
            ));
            drop(writer);
            let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
            assert_eq!(transactions.len(), 1);
            let log = format!("{transactions:?}").replace("\\\"", "\"");
            assert!(log.contains("FOR UPDATE"));
            assert!(!log.contains("UPDATE \"users\""));
            assert!(!log.contains("INSERT INTO"));
            assert!(log.contains("ROLLBACK"));
            assert!(!log.contains("COMMIT"));
            assert!(!log.contains("unit-refresh-secret"));
        }
    }

    #[tokio::test]
    async fn complete_registration_wrong_owner_invalid_or_expired_refresh_rolls_back_username() {
        for case in 0..3 {
            let id = Uuid::new_v4();
            let mut current = owner(id, "verified-hash");
            current.username = None;
            let mut proposed = current.clone();
            proposed.username = Some("requested".into());
            let mut token = refresh(id);
            match case {
                0 => token.user_id = Uuid::new_v4(),
                1 => token.is_valid = false,
                _ => token.expires_at = chrono::Utc::now() - chrono::Duration::seconds(1),
            }
            let db = Arc::new(
                MockDatabase::new(DbBackend::Postgres)
                    .append_query_results([vec![current], vec![proposed]])
                    .into_connection(),
            );
            let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
            assert!(matches!(
                writer
                    .complete_registration(
                        id,
                        Some("verified-hash".into()),
                        "requested".into(),
                        Some(token)
                    )
                    .await,
                Err(DomainError::InvalidToken)
            ));
            drop(writer);
            let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
            assert_eq!(transactions.len(), 1);
            let log = format!("{transactions:?}").replace("\\\"", "\"");
            assert!(log.find("FOR UPDATE").unwrap() < log.find("UPDATE \"users\"").unwrap());
            assert!(!log.contains("INSERT INTO"));
            assert!(log.contains("ROLLBACK"));
            assert!(!log.contains("COMMIT"));
            assert!(!log.contains("unit-refresh-secret"));
        }
    }

    #[tokio::test]
    async fn complete_registration_insert_error_rolls_back_username_and_no_session_is_returned() {
        let id = Uuid::new_v4();
        let mut current = owner(id, "verified-hash");
        current.username = None;
        let mut proposed = current.clone();
        proposed.username = Some("requested".into());
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![current], vec![proposed]])
                .append_query_errors([DbErr::Custom("unit-refresh-secret database echo".into())])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        let error = writer
            .complete_registration(
                id,
                Some("verified-hash".into()),
                "requested".into(),
                Some(refresh(id)),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, DomainError::RepositoryError(_)));
        assert!(!format!("{error:?} {error}").contains("unit-refresh-secret"));
        drop(writer);
        let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
        assert_eq!(transactions.len(), 1);
        let log = format!("{transactions:?}").replace("\\\"", "\"");
        assert!(log.find("FOR UPDATE").unwrap() < log.find("UPDATE \"users\"").unwrap());
        assert!(log.contains("INSERT INTO"));
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("COMMIT"));
        assert!(!log.contains("unit-refresh-secret"));
    }

    #[tokio::test]
    async fn reset_winning_user_lock_invalidates_previous_password_proof() {
        let id = Uuid::new_v4();
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(id, "new-password-hash")]])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        assert!(matches!(
            writer
                .issue(refresh(id), Some("previous-password-hash".into()))
                .await,
            Err(DomainError::InvalidToken)
        ));
        drop(writer);
        let log = format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log());
        assert!(log.contains("FOR UPDATE"));
        assert!(!log.contains("INSERT INTO"));
        assert!(!log.contains("unit-refresh-secret"));
    }

    #[tokio::test]
    async fn insert_failure_propagates_instead_of_returning_an_unpersisted_session() {
        let id = Uuid::new_v4();
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(id, "verified-password-hash")]])
                .append_query_errors([DbErr::Custom(
                    "unit-refresh-secret deliberately echoed by database".into(),
                )])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db);
        let error = writer
            .issue(refresh(id), Some("verified-password-hash".into()))
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("unit-refresh-secret"));
        assert!(matches!(error, DomainError::RepositoryError(_)));
    }

    #[tokio::test]
    async fn stale_authenticated_reset_never_changes_password_or_purges_sessions() {
        let id = Uuid::new_v4();
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(id, "already-reset")]])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        assert!(writer
            .change_password(id, Some("old-proof".into()), "proposed".into())
            .await
            .is_err());
        drop(writer);
        let log = format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log());
        assert!(log.contains("FOR UPDATE"));
        assert!(!log.contains("UPDATE \"users\""));
        assert!(!log.contains("DELETE"));
    }

    #[tokio::test]
    async fn password_update_and_both_global_purges_share_one_committed_transaction() {
        use sea_orm::MockExecResult;
        let id = Uuid::new_v4();
        let mut updated = owner(id, "new-hash");
        updated.updated_at = chrono::Utc::now().naive_utc();
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(id, "old-hash")], vec![updated]])
                .append_exec_results([
                    MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 3,
                    },
                    MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 2,
                    },
                ])
                .into_connection(),
        );
        let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
        writer
            .change_password(id, Some("old-hash".into()), "new-hash".into())
            .await
            .unwrap();
        drop(writer);
        let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
        assert_eq!(transactions.len(), 1);
        let log = format!("{transactions:?}");
        assert!(log.contains("FOR UPDATE"));
        assert!(log.contains("refresh_tokens"));
        assert!(log.contains("password_reset_tokens"));
        assert_eq!(log.matches("DELETE FROM").count(), 2);
        assert!(log.contains("COMMIT"));
    }
}
