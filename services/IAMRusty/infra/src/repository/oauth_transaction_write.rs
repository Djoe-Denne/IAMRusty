//! OAuth transaction storage is always on the shared writer (ADR-0412).

use async_trait::async_trait;
use iam_domain::{
    entity::oauth_transaction::{ConsumeOAuthTransaction, OAuthTransaction, OAuthTransactionError},
    port::repository::OAuthTransactionWriteRepository,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};
use std::sync::Arc;

pub struct SeaOrmOAuthTransactionWriteRepository {
    writer: Arc<DatabaseConnection>,
}

impl SeaOrmOAuthTransactionWriteRepository {
    #[must_use]
    pub const fn new(writer: Arc<DatabaseConnection>) -> Self {
        Self { writer }
    }
}

fn storage_error(_: sea_orm::DbErr) -> OAuthTransactionError {
    OAuthTransactionError::Storage
}

fn hash_bytes(row: &sea_orm::QueryResult, name: &str) -> Result<[u8; 32], OAuthTransactionError> {
    let bytes: Vec<u8> = row.try_get("", name).map_err(storage_error)?;
    bytes.try_into().map_err(|_| OAuthTransactionError::Storage)
}

/// Bind unix seconds as `timestamptz` so Postgres never sees an `i64 as f64`.
fn unix_seconds_timestamptz(
    unix_seconds: i64,
) -> Result<chrono::DateTime<chrono::FixedOffset>, OAuthTransactionError> {
    chrono::DateTime::from_timestamp(unix_seconds, 0)
        .map(|utc| utc.fixed_offset())
        .ok_or(OAuthTransactionError::InvalidTransaction)
}

#[async_trait]
impl OAuthTransactionWriteRepository for SeaOrmOAuthTransactionWriteRepository {
    async fn purge_expired(&self, batch_size: u32) -> Result<u64, OAuthTransactionError> {
        if !(1..=1000).contains(&batch_size) {
            return Err(OAuthTransactionError::InvalidTransaction);
        }
        // One autocommit statement: locks and delete share a short atomic operation.
        // Include consumed AND unconsumed rows; valid rows retain their replay marker.
        let result = self.writer.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "WITH expired AS (SELECT id FROM oauth_transactions WHERE expires_at<=clock_timestamp() ORDER BY expires_at,id LIMIT $1 FOR UPDATE SKIP LOCKED) DELETE FROM oauth_transactions WHERE id IN (SELECT id FROM expired)",
            vec![i64::from(batch_size).into()],
        )).await.map_err(storage_error)?;
        Ok(result.rows_affected())
    }

    async fn create(&self, tx: &OAuthTransaction) -> Result<(), OAuthTransactionError> {
        if tx.consumed_at.is_some() || tx.expires_at <= chrono::Utc::now().timestamp() {
            return Err(OAuthTransactionError::InvalidTransaction);
        }
        self.writer.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO oauth_transactions (id,nonce_hash,state_hash,browser_nonce_hash,provider,operation,target_user_id,redirect_uri,pkce_verifier,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            vec![tx.id.into(), tx.nonce_hash.to_vec().into(), tx.state_hash.to_vec().into(),
                tx.browser_nonce_hash.to_vec().into(), tx.provider.as_str().into(),
                tx.operation.as_str().into(), tx.operation.target_user_id().into(),
                tx.redirect_uri.clone().into(), tx.pkce_verifier.clone().into(), unix_seconds_timestamptz(tx.expires_at)?.into()]))
            .await.map_err(storage_error)?;
        Ok(())
    }

    async fn consume(
        &self,
        input: &ConsumeOAuthTransaction,
    ) -> Result<Option<OAuthTransaction>, OAuthTransactionError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        // Lock the matching row before removing the secret. All bindings and
        // replay/expiry predicates are on the writer, not a replica snapshot.
        let row = tx.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id,nonce_hash,state_hash,browser_nonce_hash,pkce_verifier FROM oauth_transactions WHERE nonce_hash=$1 AND state_hash=$2 AND browser_nonce_hash=$3 AND provider=$4 AND operation=$5 AND target_user_id IS NOT DISTINCT FROM $6::uuid AND redirect_uri=$7 AND expires_at=$8 AND consumed_at IS NULL AND expires_at>clock_timestamp() FOR UPDATE",
            vec![input.nonce_hash.to_vec().into(), input.state_hash.to_vec().into(), input.browser_nonce_hash.to_vec().into(),
                input.provider.as_str().into(), input.operation.as_str().into(), input.operation.target_user_id().into(),
                input.redirect_uri.clone().into(), unix_seconds_timestamptz(input.expires_at)?.into()]))
            .await.map_err(storage_error)?;
        let Some(row) = row else {
            tx.commit().await.map_err(storage_error)?;
            return Ok(None);
        };
        let id: uuid::Uuid = row.try_get("", "id").map_err(storage_error)?;
        let verifier: Option<String> = row.try_get("", "pkce_verifier").map_err(storage_error)?;
        tx.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE oauth_transactions SET consumed_at=clock_timestamp(),pkce_verifier=NULL WHERE id=$1", vec![id.into()]))
            .await.map_err(storage_error)?;
        let transaction = OAuthTransaction {
            id,
            nonce_hash: hash_bytes(&row, "nonce_hash")?,
            state_hash: hash_bytes(&row, "state_hash")?,
            browser_nonce_hash: hash_bytes(&row, "browser_nonce_hash")?,
            provider: input.provider.clone(),
            operation: input.operation.clone(),
            redirect_uri: input.redirect_uri.clone(),
            expires_at: input.expires_at,
            consumed_at: Some(chrono::Utc::now().timestamp()),
            pkce_verifier: verifier,
        };
        tx.commit().await.map_err(storage_error)?;
        Ok(Some(transaction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iam_domain::entity::oauth_transaction::OAuthOperation;
    use sea_orm::{MockDatabase, MockExecResult};

    #[tokio::test]
    async fn purge_rejects_invalid_cap_before_any_sql() {
        let db = Arc::new(MockDatabase::new(DbBackend::Postgres).into_connection());
        let repository = SeaOrmOAuthTransactionWriteRepository::new(db.clone());
        for cap in [0, 1001, u32::MAX] {
            assert!(matches!(
                repository.purge_expired(cap).await,
                Err(OAuthTransactionError::InvalidTransaction)
            ));
        }
        drop(repository);
        assert!(Arc::try_unwrap(db)
            .unwrap()
            .into_transaction_log()
            .is_empty());
    }

    #[tokio::test]
    async fn purge_is_one_bounded_atomic_primary_clock_statement_without_verifier_payload() {
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 3,
                }])
                .into_connection(),
        );
        let repository = SeaOrmOAuthTransactionWriteRepository::new(db.clone());
        assert_eq!(repository.purge_expired(3).await.unwrap(), 3);
        drop(repository);
        let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
        assert_eq!(transactions.len(), 1);
        let log = format!("{transactions:?}");
        assert!(log.contains("expires_at<=clock_timestamp()"));
        assert!(log.contains("ORDER BY expires_at,id LIMIT $1 FOR UPDATE SKIP LOCKED"));
        assert!(log.contains("DELETE FROM oauth_transactions WHERE id IN (SELECT id FROM expired)"));
        assert!(!log.contains("consumed_at IS"));
        assert!(!log.contains("pkce_verifier"));
        assert!(!log.contains("RETURNING"));
    }

    #[tokio::test]
    async fn purge_storage_error_is_log_safe_and_next_call_can_recover() {
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_exec_errors([sea_orm::DbErr::Custom("pkce-secret state-secret".into())])
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 0,
                }])
                .into_connection(),
        );
        let repository = SeaOrmOAuthTransactionWriteRepository::new(db);
        let error = repository.purge_expired(100).await.unwrap_err();
        assert!(matches!(error, OAuthTransactionError::Storage));
        assert!(!format!("{error:?} {error}").contains("secret"));
        assert_eq!(repository.purge_expired(100).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn consume_commits_browser_bindings_and_verifier_erasure_before_return() {
        let now = chrono::Utc::now();
        let expires = now.timestamp() + 300;
        let row = crate::repository::entity::oauth_transactions::Model {
            id: uuid::Uuid::new_v4(),
            nonce_hash: vec![1; 32],
            state_hash: vec![2; 32],
            browser_nonce_hash: vec![3; 32],
            provider: "github".into(),
            operation: "login".into(),
            target_user_id: None,
            redirect_uri: "https://iam.example/callback".into(),
            pkce_verifier: Some("unit-pkce-verifier".into()),
            expires_at: chrono::DateTime::from_timestamp(expires, 0)
                .unwrap()
                .fixed_offset(),
            consumed_at: None,
        };
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![row]])
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 1,
                }])
                .into_connection(),
        );
        let repository = SeaOrmOAuthTransactionWriteRepository::new(db.clone());
        let result = repository
            .consume(&ConsumeOAuthTransaction {
                nonce_hash: [1; 32],
                state_hash: [2; 32],
                browser_nonce_hash: [3; 32],
                provider: "github".parse().unwrap(),
                operation: OAuthOperation::Login,
                redirect_uri: "https://iam.example/callback".into(),
                expires_at: expires,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.pkce_verifier.as_deref(), Some("unit-pkce-verifier"));
        assert!(result.consumed_at.is_some());
        drop(repository);
        let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
        assert_eq!(transactions.len(), 1);
        let log = format!("{transactions:?}");
        assert!(log.contains("FOR UPDATE"));
        assert!(log.contains("browser_nonce_hash"));
        assert!(log.contains("consumed_at IS NULL"));
        assert!(log.contains("clock_timestamp()"));
        assert!(log.contains("pkce_verifier=NULL"));
        assert!(log.contains("COMMIT"));
        assert!(!log.contains("unit-pkce-verifier"));
    }
}
