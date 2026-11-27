//! Authoritative primary-store session-liveness adapter for IAM.

use async_trait::async_trait;
use rustycog::http::{
    SessionLivenessChecker, SessionLivenessDecision, SessionLivenessError, SessionLivenessOutcome,
    SessionLivenessQuery,
};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, Statement, TransactionTrait,
};
use std::sync::Arc;
use uuid::Uuid;

use super::authentication_session::read_epoch;
use super::entity::users::Entity as Users;

/// Reads the current account epoch from the authoritative primary connection.
///
/// The adapter performs no caching and never falls back to a read replica or
/// JWT-only validation.
#[derive(Clone)]
pub struct SeaOrmSessionLivenessChecker {
    db: Arc<DatabaseConnection>,
    platform_issuer: String,
}

impl SeaOrmSessionLivenessChecker {
    /// Construct a checker bound to the primary writer connection.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>, platform_issuer: String) -> Self {
        Self {
            db,
            platform_issuer,
        }
    }

    /// Persist a marker on the same writer and in the same stream as account epochs.
    ///
    /// # Errors
    ///
    /// Returns a database error when the writer cannot commit the marker.
    pub async fn write_checkpoint(&self, challenge: Uuid) -> Result<bool, sea_orm::DbErr> {
        let tx = self.db.begin().await?;
        // All IAM instances sharing this writer use this transaction-scoped lock.
        // Contention refuses the probe instead of queuing marker writes.
        let locked = tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT pg_try_advisory_xact_lock(20847615, 317) AND NOT pg_is_in_recovery() AS acquired"))
            .await?
            .ok_or_else(|| sea_orm::DbErr::Custom("missing checkpoint lock result".into()))?
            .try_get::<bool>("", "acquired")?;
        if !locked {
            tx.rollback().await?;
            return Ok(false);
        }
        let inserted = tx
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "WITH added AS (
                INSERT INTO public.iam_session_liveness_checkpoints (challenge)
                VALUES ($1) ON CONFLICT DO NOTHING RETURNING challenge
             )
             INSERT INTO public.iam_session_liveness_checkpoint_queue (challenge)
             SELECT challenge FROM added",
                [challenge.into()],
            ))
            .await?;
        if inserted.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }
        tx.execute_unprepared(
            "WITH retired AS (
                SELECT challenge FROM public.iam_session_liveness_checkpoint_queue
                ORDER BY sequence DESC OFFSET 128
             )
             DELETE FROM public.iam_session_liveness_checkpoints
             WHERE challenge IN (SELECT challenge FROM retired)",
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}

#[async_trait]
impl SessionLivenessChecker for SeaOrmSessionLivenessChecker {
    async fn check(
        &self,
        query: &SessionLivenessQuery,
    ) -> Result<SessionLivenessOutcome, SessionLivenessError> {
        if query.issuer != self.platform_issuer {
            return Ok(SessionLivenessOutcome::new(
                SessionLivenessDecision::NotApplicable,
            ));
        }
        let user = Users::find_by_id(query.subject)
            .one(self.db.as_ref())
            .await
            .map_err(|_| SessionLivenessError::unavailable("primary epoch read failed"))?;
        let Some(user) = user else {
            return Ok(SessionLivenessOutcome::new(
                SessionLivenessDecision::Revoked,
            ));
        };
        let current_epoch = read_epoch(&user.auth_epoch)
            .map_err(|_| SessionLivenessError::unavailable("stored epoch is invalid"))?;
        if query.auth_epoch == Some(current_epoch)
            || (query.auth_epoch.is_none() && current_epoch == 0)
        {
            Ok(SessionLivenessOutcome::with_current_epoch(
                SessionLivenessDecision::Live,
                current_epoch,
            ))
        } else {
            Ok(SessionLivenessOutcome::with_current_epoch(
                SessionLivenessDecision::Revoked,
                current_epoch,
            ))
        }
    }
}
