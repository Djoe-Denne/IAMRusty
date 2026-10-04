use async_trait::async_trait;
use iam_domain::entity::token::RefreshToken as DomainRefreshToken;
use iam_domain::port::repository::RefreshTokenWriteRepository;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::authentication_session::lock_user;
use super::entity::{prelude::RefreshTokens, refresh_tokens};

/// `SeaORM` implementation of `RefreshTokenWriteRepository`
#[derive(Clone)]
pub struct RefreshTokenWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RefreshTokenWriteRepositoryImpl {
    /// Create a new `RefreshTokenWriteRepositoryImpl`
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a domain refresh token to a database model
    fn to_model(token: &DomainRefreshToken) -> refresh_tokens::ActiveModel {
        refresh_tokens::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token: Set(DomainRefreshToken::hash_token(&token.token)),
            is_valid: Set(token.is_valid),
            created_at: Set(token.created_at.into()),
            expires_at: Set(token.expires_at.into()),
        }
    }

    /// Convert a database model to a domain refresh token
    fn to_domain(model: refresh_tokens::Model) -> DomainRefreshToken {
        DomainRefreshToken {
            id: model.id,
            user_id: model.user_id,
            token: model.token,
            is_valid: model.is_valid,
            created_at: model.created_at.into(),
            expires_at: model.expires_at.into(),
        }
    }
}

#[async_trait]
impl RefreshTokenWriteRepository for RefreshTokenWriteRepositoryImpl {
    type Error = DbErr;

    async fn create(&self, token: DomainRefreshToken) -> Result<DomainRefreshToken, Self::Error> {
        debug!("Creating new refresh token for user ID: {}", token.user_id);

        let model = Self::to_model(&token);
        let txn = self.db.begin().await?;
        lock_user(&txn, token.user_id).await?;
        let res = model.insert(&txn).await?;
        txn.commit().await?;

        Ok(Self::to_domain(res))
    }

    async fn update_validity(&self, token_id: Uuid, is_valid: bool) -> Result<(), Self::Error> {
        debug!(
            "Updating refresh token validity: id={}, is_valid={}",
            token_id, is_valid
        );

        let token = RefreshTokens::find_by_id(token_id)
            .one(self.db.as_ref())
            .await?;

        if let Some(token) = token {
            let txn = self.db.begin().await?;
            lock_user(&txn, token.user_id).await?;
            // Re-read after the owner lock: reset/rotation may already have removed it.
            let Some(current) = RefreshTokens::find_by_id(token_id).one(&txn).await? else {
                return Ok(());
            };
            let mut model = refresh_tokens::ActiveModel::from(current);
            model.is_valid = Set(is_valid);

            model.update(&txn).await?;
            txn.commit().await?;
            debug!("Updated refresh token validity");
        } else {
            debug!("Refresh token not found for update: {}", token_id);
        }

        Ok(())
    }

    async fn delete_by_id(&self, token_id: Uuid) -> Result<(), Self::Error> {
        debug!("Deleting refresh token by ID: {}", token_id);

        let txn = self.db.begin().await?;
        if let Some(token) = RefreshTokens::find_by_id(token_id).one(&txn).await? {
            lock_user(&txn, token.user_id).await?;
        }
        let result = RefreshTokens::delete_by_id(token_id).exec(&txn).await?;
        txn.commit().await?;

        if result.rows_affected > 0 {
            debug!("Deleted refresh token: {}", token_id);
        } else {
            debug!("Refresh token not found for deletion: {}", token_id);
        }

        Ok(())
    }

    async fn rotate(
        &self,
        old_token_id: Uuid,
        new_token: DomainRefreshToken,
    ) -> Result<DomainRefreshToken, Self::Error> {
        debug!(
            "Rotating refresh token: old_id={}, new_id={}",
            old_token_id, new_token.id
        );

        let txn = self.db.begin().await?;
        lock_user(&txn, new_token.user_id).await?;
        let old = RefreshTokens::find_by_id(old_token_id)
            .one(&txn)
            .await?
            .filter(|old| {
                old.user_id == new_token.user_id
                    && old.is_valid
                    && old.expires_at > chrono::Utc::now()
            })
            .ok_or_else(|| DbErr::RecordNotFound("Refresh token not found".into()))?;
        if !new_token.is_valid || new_token.expires_at <= chrono::Utc::now() {
            return Err(DbErr::RecordNotFound("Refresh token not found".into()));
        }
        let model = Self::to_model(&new_token);
        let inserted = model.insert(&txn).await?;

        let delete_result = RefreshTokens::delete_by_id(old.id).exec(&txn).await?;

        if delete_result.rows_affected == 0 {
            txn.rollback().await?;
            return Err(DbErr::RecordNotFound(format!(
                "Refresh token not found: {old_token_id}"
            )));
        }

        txn.commit().await?;
        Ok(Self::to_domain(inserted))
    }

    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        debug!("Deleting all refresh tokens for user ID: {}", user_id);

        let txn = self.db.begin().await?;
        lock_user(&txn, user_id).await?;
        let result = RefreshTokens::delete_many()
            .filter(refresh_tokens::Column::UserId.eq(user_id))
            .exec(&txn)
            .await?;
        txn.commit().await?;

        debug!("Deleted {} refresh tokens", result.rows_affected);

        Ok(result.rows_affected)
    }
}

#[cfg(test)]
mod atomic_writer_tests {
    use super::super::entity::users;
    use super::*;
    use chrono::{Duration, Utc};
    use sea_orm::{DbBackend, MockDatabase, MockExecResult};

    fn owner(id: Uuid) -> users::Model {
        let now = Utc::now().naive_utc();
        users::Model {
            id,
            username: Some("registered".into()),
            password_hash: Some("hash".into()),
            avatar_url: None,
            created_at: now,
            updated_at: now,
        }
    }
    fn token(user_id: Uuid) -> DomainRefreshToken {
        let now = Utc::now();
        DomainRefreshToken {
            id: Uuid::new_v4(),
            user_id,
            token: "rotation-secret".into(),
            is_valid: true,
            created_at: now,
            expires_at: now + Duration::hours(1),
        }
    }
    fn stored(token: &DomainRefreshToken) -> refresh_tokens::Model {
        refresh_tokens::Model {
            id: token.id,
            user_id: token.user_id,
            token: DomainRefreshToken::hash_token(&token.token),
            is_valid: token.is_valid,
            created_at: token.created_at.into(),
            expires_at: token.expires_at.into(),
        }
    }
    fn log(db: Arc<DatabaseConnection>) -> (usize, String) {
        let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
        (
            transactions.len(),
            format!("{transactions:?}").replace("\\\"", "\""),
        )
    }
    fn assert_rollback(log: &str) {
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("COMMIT"));
        assert!(!log.contains("rotation-secret"));
    }

    #[tokio::test]
    async fn rotate_locks_owner_inserts_hash_and_removes_old_in_one_commit() {
        let user_id = Uuid::new_v4();
        let old = token(user_id);
        let new = token(user_id);
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(user_id)]])
                .append_query_results([vec![stored(&old)], vec![stored(&new)]])
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 1,
                }])
                .into_connection(),
        );
        let writer = RefreshTokenWriteRepositoryImpl::new(db.clone());
        let result = writer.rotate(old.id, new.clone()).await.unwrap();
        assert_eq!(result.id, new.id);
        assert_eq!(result.token, DomainRefreshToken::hash_token(&new.token));
        drop(writer);
        let (count, log) = log(db);
        assert_eq!(count, 1);
        let lock = log.find("FOR UPDATE").unwrap();
        let insert = log.find("INSERT INTO").unwrap();
        let delete = log.find("DELETE FROM").unwrap();
        assert!(lock < insert && insert < delete && delete < log.find("COMMIT").unwrap());
        assert!(!log.contains("ROLLBACK"));
        assert!(!log.contains("rotation-secret"));
    }

    #[tokio::test]
    async fn rotate_missing_wrong_owner_invalid_or_expired_never_inserts() {
        for case in 0..=5 {
            let user_id = Uuid::new_v4();
            let mut old = token(user_id);
            let mut new = token(user_id);
            match case {
                1 => old.user_id = Uuid::new_v4(),
                2 => old.is_valid = false,
                3 => old.expires_at = Utc::now() - Duration::seconds(1),
                4 => new.is_valid = false,
                5 => new.expires_at = Utc::now() - Duration::seconds(1),
                _ => {}
            }
            let rows = if case == 0 {
                vec![]
            } else {
                vec![stored(&old)]
            };
            let db = Arc::new(
                MockDatabase::new(DbBackend::Postgres)
                    .append_query_results([vec![owner(user_id)]])
                    .append_query_results([rows])
                    .into_connection(),
            );
            let writer = RefreshTokenWriteRepositoryImpl::new(db.clone());
            assert!(matches!(
                writer.rotate(old.id, new).await,
                Err(DbErr::RecordNotFound(_))
            ));
            drop(writer);
            let (_, log) = log(db);
            assert!(log.contains("FOR UPDATE"));
            assert!(!log.contains("INSERT INTO"));
            assert!(!log.contains("DELETE FROM"));
            assert_rollback(&log);
        }
    }

    #[tokio::test]
    async fn rotate_zero_row_delete_explicitly_rolls_back_orphan_insert() {
        let user_id = Uuid::new_v4();
        let old = token(user_id);
        let new = token(user_id);
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(user_id)]])
                .append_query_results([vec![stored(&old)], vec![stored(&new)]])
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 0,
                }])
                .into_connection(),
        );
        let writer = RefreshTokenWriteRepositoryImpl::new(db.clone());
        assert!(matches!(
            writer.rotate(old.id, new).await,
            Err(DbErr::RecordNotFound(_))
        ));
        drop(writer);
        let (count, log) = log(db);
        assert_eq!(count, 1);
        assert!(log.contains("FOR UPDATE"));
        assert!(log.contains("INSERT INTO"));
        assert!(log.contains("DELETE FROM"));
        assert_rollback(&log);
    }

    #[tokio::test]
    async fn rotate_insert_or_delete_error_never_commits_partial_rotation() {
        for insert_error in [false, true] {
            let user_id = Uuid::new_v4();
            let old = token(user_id);
            let new = token(user_id);
            let mock = MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![owner(user_id)]])
                .append_query_results([vec![stored(&old)]]);
            let mock = if insert_error {
                mock.append_query_errors([DbErr::Custom("injected insert failure".into())])
            } else {
                mock.append_query_results([vec![stored(&new)]])
                    .append_exec_errors([DbErr::Custom("injected delete failure".into())])
            };
            let db = Arc::new(mock.into_connection());
            let writer = RefreshTokenWriteRepositoryImpl::new(db.clone());
            assert!(writer.rotate(old.id, new).await.is_err());
            drop(writer);
            let (_, log) = log(db);
            assert!(log.contains("FOR UPDATE"));
            assert!(log.contains("INSERT INTO"));
            assert_eq!(log.contains("DELETE FROM"), !insert_error);
            assert_rollback(&log);
        }
    }

    #[tokio::test]
    async fn update_validity_rereads_after_owner_lock_and_missing_row_is_not_resurrected() {
        for disappeared in [false, true] {
            let user_id = Uuid::new_v4();
            let old = token(user_id);
            let current = stored(&old);
            let mut initial = current.clone();
            initial.token = "stale-hash".into();
            let mock = MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![initial]])
                .append_query_results([vec![owner(user_id)]]);
            let mock = if disappeared {
                mock.append_query_results([Vec::<refresh_tokens::Model>::new()])
            } else {
                let mut updated = current.clone();
                updated.is_valid = false;
                mock.append_query_results([vec![current], vec![updated]])
            };
            let db = Arc::new(mock.into_connection());
            let writer = RefreshTokenWriteRepositoryImpl::new(db.clone());
            writer.update_validity(old.id, false).await.unwrap();
            drop(writer);
            let (_, log) = log(db);
            assert_eq!(log.matches("SELECT \"refresh_tokens\"").count(), 2);
            let lock = log.find("FOR UPDATE").unwrap();
            let reread = log.rfind("SELECT \"refresh_tokens\"").unwrap();
            assert!(lock < reread);
            assert_eq!(log.contains("UPDATE \"refresh_tokens\""), !disappeared);
            if disappeared {
                assert_rollback(&log);
            } else {
                assert!(reread < log.find("UPDATE \"refresh_tokens\"").unwrap());
                assert!(log.contains("COMMIT"));
                // A stale snapshot cannot write unrelated fields back.
                let update = &log[log.find("UPDATE \"refresh_tokens\"").unwrap()..];
                assert!(!update.contains("stale-hash"));
            }
            assert!(!log.contains("rotation-secret"));
        }
    }

    #[tokio::test]
    async fn both_remove_paths_lock_owner_before_delete_and_commit() {
        for by_user in [false, true] {
            let user_id = Uuid::new_v4();
            let old = token(user_id);
            let mock = MockDatabase::new(DbBackend::Postgres);
            let mock = if by_user {
                mock
            } else {
                mock.append_query_results([vec![stored(&old)]])
            };
            let db = Arc::new(
                mock.append_query_results([vec![owner(user_id)]])
                    .append_exec_results([MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 2,
                    }])
                    .into_connection(),
            );
            let writer = RefreshTokenWriteRepositoryImpl::new(db.clone());
            if by_user {
                assert_eq!(writer.delete_by_user_id(user_id).await.unwrap(), 2);
            } else {
                writer.delete_by_id(old.id).await.unwrap();
            }
            drop(writer);
            let (count, log) = log(db);
            assert_eq!(count, 1);
            assert!(log.find("FOR UPDATE").unwrap() < log.find("DELETE FROM").unwrap());
            assert!(log.contains("COMMIT"));
            assert!(!log.contains("rotation-secret"));
        }
    }
}
