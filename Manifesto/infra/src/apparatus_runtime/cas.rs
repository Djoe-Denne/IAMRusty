//! Claim et écriture observed (CAS generation + epoch + owner + expiry).

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use uuid::Uuid;

use super::APPARATUS_LEASE_TTL;

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
                 observed_digest = $2 \
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

fn truncate_owner(owner: &str) -> String {
    owner.chars().take(64).collect()
}
