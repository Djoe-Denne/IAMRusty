//! Claim, écriture observed et writer d'échec fencé (retry / backoff / terminal).

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use uuid::Uuid;

use super::{next_backoff, APPARATUS_ERROR_BIND_FAILED, APPARATUS_LEASE_TTL};

/// Claim atomique : `lease_epoch + 1`, owner, expiry = now + TTL.
///
/// # Errors
///
/// Retourne [`DbErr`] si l'UPDATE échoue.
pub async fn claim_binding(
    db: &DatabaseConnection,
    component_id: Uuid,
    owner: &str,
    now: DateTime<Utc>,
) -> Result<Option<i64>, DbErr> {
    let owner = truncate_owner(owner);
    let expires = now + chrono::Duration::seconds(APPARATUS_LEASE_TTL.as_secs() as i64);
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE apparatus_bindings \
             SET lease_epoch = lease_epoch + 1, \
                 lease_owner = $1, \
                 lease_expires_at = $2 \
             WHERE component_id = $3 \
               AND source = 'managed' \
               AND desired_generation > observed_generation \
               AND (lease_owner = '' OR lease_owner = $1 OR lease_expires_at <= $4) \
             RETURNING lease_epoch",
            [
                owner.into(),
                expires.into(),
                component_id.into(),
                now.into(),
            ],
        ))
        .await?;
    match row {
        Some(row) => Ok(Some(row.try_get("", "lease_epoch")?)),
        None => Ok(None),
    }
}

/// Écrit `observed_generation` / `observed_digest` seulement si le writer est encore valide.
///
/// 0 ligne = refuse (generation, epoch, owner ou expiry).
///
/// # Errors
///
/// Retourne [`DbErr`] si l'UPDATE échoue.
pub async fn write_observed(
    db: &DatabaseConnection,
    component_id: Uuid,
    generation: i64,
    digest: &str,
    epoch: i64,
    owner: &str,
    now: DateTime<Utc>,
) -> Result<u64, DbErr> {
    let owner = truncate_owner(owner);
    let result = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE apparatus_bindings \
             SET observed_generation = $1, \
                 observed_digest = $2, \
                 retry_count = 0, \
                 last_error_code = NULL, \
                 next_retry_at = NULL \
             WHERE component_id = $3 \
               AND desired_generation = $1 \
               AND lease_epoch = $4 \
               AND lease_owner = $5 \
               AND lease_expires_at > $6",
            [
                generation.into(),
                digest.into(),
                component_id.into(),
                epoch.into(),
                owner.into(),
                now.into(),
            ],
        ))
        .await?;
    Ok(result.rows_affected())
}

/// Écrit retry / backoff / terminal seulement si le writer est encore valide.
///
/// 0 ligne = refuse (generation, epoch, owner, expiry) : **pas** d'incrément.
/// `last_error_code` = `bind_failed` (jamais `error.to_string()`).
///
/// # Errors
///
/// Retourne [`DbErr`] si le SELECT ou l'UPDATE échoue.
pub async fn write_bind_failure(
    db: &DatabaseConnection,
    component_id: Uuid,
    generation: i64,
    epoch: i64,
    owner: &str,
    now: DateTime<Utc>,
) -> Result<u64, DbErr> {
    let owner = truncate_owner(owner);
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT retry_count FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await?
    else {
        return Ok(0);
    };
    let current: i32 = row.try_get("", "retry_count")?;
    let (new_count, next_retry_at) = next_backoff(current, now);
    let result = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE apparatus_bindings \
             SET retry_count = $1, \
                 last_error_code = $2, \
                 next_retry_at = $3 \
             WHERE component_id = $4 \
               AND desired_generation = $5 \
               AND lease_epoch = $6 \
               AND lease_owner = $7 \
               AND lease_expires_at > $8 \
               AND retry_count = $9",
            [
                new_count.into(),
                APPARATUS_ERROR_BIND_FAILED.into(),
                next_retry_at.into(),
                component_id.into(),
                generation.into(),
                epoch.into(),
                owner.into(),
                now.into(),
                current.into(),
            ],
        ))
        .await?;
    Ok(result.rows_affected())
}

fn truncate_owner(owner: &str) -> String {
    owner.chars().take(64).collect()
}
