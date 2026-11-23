use std::sync::Arc;

use async_trait::async_trait;
use iam_application::usecase::profile_email_verification::{
    ProfileEmailVerificationRepository, ProfileVerificationError, SavedAddress, VerificationResult,
};
use iam_domain::entity::events::ProfileEmailVerificationRequested;
use rustycog::outbox::OutboxRecorder;
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction, IsolationLevel,
    QueryResult, Statement, TransactionTrait, Value,
};
use uuid::Uuid;

pub struct PostgresProfileEmailVerification {
    writer: Arc<DatabaseConnection>,
}

impl PostgresProfileEmailVerification {
    #[must_use]
    pub const fn new(writer: Arc<DatabaseConnection>) -> Self {
        Self { writer }
    }
}

fn sql<const N: usize>(statement: &str, params: [Value; N]) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, statement, params)
}

fn writer_failure<T, E>(result: Result<T, E>) -> VerificationResult<T> {
    result.map_err(|_| ProfileVerificationError::Internal)
}

/// Both actions lock the saved address first. The post-lock checks below
/// serialize across instances and against address replacement/removal.
async fn lock_unverified_address(
    txn: &DatabaseTransaction,
    user_id: Uuid,
    address_id: Uuid,
) -> VerificationResult<String> {
    let row = writer_failure(
        txn.query_one(sql(
            "SELECT email FROM user_emails
             WHERE id = $1 AND user_id = $2 AND NOT is_verified FOR UPDATE",
            [address_id.into(), user_id.into()],
        ))
        .await,
    )?
    .ok_or(ProfileVerificationError::NotEligible)?;
    writer_failure(row.try_get("", "email"))
}

fn project(row: QueryResult) -> VerificationResult<SavedAddress> {
    Ok(SavedAddress {
        id: writer_failure(row.try_get("", "id"))?,
        email: writer_failure(row.try_get("", "email"))?,
        is_primary: writer_failure(row.try_get("", "is_primary"))?,
        is_verified: writer_failure(row.try_get("", "is_verified"))?,
    })
}

#[async_trait]
impl ProfileEmailVerificationRepository for PostgresProfileEmailVerification {
    async fn list(&self, user_id: Uuid) -> VerificationResult<Vec<SavedAddress>> {
        writer_failure(
            self.writer
                .query_all(sql(
                    r#"SELECT e.id, e.email, e.is_primary,
                              (e.is_verified OR p.confirmed_at IS NOT NULL) AS is_verified
                       FROM user_emails e LEFT JOIN profile_email_verifications p
                            ON p.address_id = e.id AND p.user_id = e.user_id
                            AND p.email = e.email COLLATE "C"
                       WHERE e.user_id = $1"#,
                    [user_id.into()],
                ))
                .await,
        )?
        .into_iter()
        .map(project)
        .collect()
    }

    async fn issue(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        digest: [u8; 32],
        verification_url: String,
    ) -> VerificationResult<()> {
        let txn = writer_failure(
            self.writer
                .begin_with_config(Some(IsolationLevel::ReadCommitted), None)
                .await,
        )?;
        let email = lock_unverified_address(&txn, user_id, address_id).await?;
        if let Some(current) = writer_failure(
            txn.query_one(sql(
                "SELECT confirmed_at IS NOT NULL AS confirmed,
                        issued_at + interval '60 seconds' > clock_timestamp() AS cooling_off
                 FROM profile_email_verifications
                 WHERE address_id = $1 AND user_id = $2 AND email = $3",
                [address_id.into(), user_id.into(), email.as_str().into()],
            ))
            .await,
        )? {
            if writer_failure(current.try_get::<bool>("", "confirmed"))? {
                return Err(ProfileVerificationError::NotEligible);
            }
            if writer_failure(current.try_get::<bool>("", "cooling_off"))? {
                return Err(ProfileVerificationError::Cooldown);
            }
        }
        writer_failure(
            txn.execute(sql(
                "WITH stamp AS (SELECT clock_timestamp() AS stamped_at)
                 INSERT INTO profile_email_verifications
                     (address_id, user_id, email, token_digest, issued_at, expires_at, confirmed_at)
                 SELECT $1, $2, $3, $4, stamped_at, stamped_at + interval '24 hours', NULL
                 FROM stamp WHERE true
                 ON CONFLICT (address_id) DO UPDATE SET
                     user_id = EXCLUDED.user_id, email = EXCLUDED.email,
                     token_digest = EXCLUDED.token_digest, issued_at = EXCLUDED.issued_at,
                     expires_at = EXCLUDED.expires_at, confirmed_at = NULL",
                [
                    address_id.into(),
                    user_id.into(),
                    email.as_str().into(),
                    digest.to_vec().into(),
                ],
            ))
            .await,
        )?;
        let event = ProfileEmailVerificationRequested::new(user_id, email, verification_url);
        writer_failure(OutboxRecorder.record(&txn, &event).await)?;
        writer_failure(txn.commit().await)
    }

    async fn confirm(&self, user_id: Uuid, digest: [u8; 32]) -> VerificationResult<()> {
        let txn = writer_failure(
            self.writer
                .begin_with_config(Some(IsolationLevel::ReadCommitted), None)
                .await,
        )?;
        let digest = digest.to_vec();
        let row = writer_failure(
            txn.query_one(sql(
                "SELECT address_id FROM profile_email_verifications
                 WHERE token_digest = $1 AND user_id = $2 AND confirmed_at IS NULL",
                [digest.clone().into(), user_id.into()],
            ))
            .await,
        )?
        .ok_or(ProfileVerificationError::NotEligible)?;
        let address_id = writer_failure(row.try_get::<Uuid>("", "address_id"))?;
        let email = lock_unverified_address(&txn, user_id, address_id).await?;
        let updated = writer_failure(
            txn.execute(sql(
                "WITH stamp AS (SELECT clock_timestamp() AS stamped_at)
                 UPDATE profile_email_verifications SET confirmed_at = stamped_at FROM stamp
                 WHERE address_id = $1 AND user_id = $2 AND email = $3
                   AND token_digest = $4 AND confirmed_at IS NULL AND expires_at > stamped_at",
                [
                    address_id.into(),
                    user_id.into(),
                    email.into(),
                    digest.into(),
                ],
            ))
            .await,
        )?;
        if updated.rows_affected() != 1 {
            return Err(ProfileVerificationError::NotEligible);
        }
        writer_failure(txn.commit().await)
    }
}
