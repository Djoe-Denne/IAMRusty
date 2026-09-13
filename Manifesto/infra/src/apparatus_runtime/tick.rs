//! Scan des bindings dûs et application via le port P0.

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use uuid::Uuid;

use apparatus_contracts::{ApparatusId, ApparatusRuntime, BindRequest, BindingId, ReleaseDigest};

use std::sync::Arc;
use std::time::Duration;

use super::cas::{claim_binding, write_observed};
use super::cleanup::apply_cleanup_due;
use super::derived_operation_id;
use crate::apparatus_mapping::legacy_to_apparatus;

/// Échec d'un passage d'application.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeApplyError {
    /// Erreur SQL.
    #[error("apparatus apply db: {0}")]
    Db(#[from] DbErr),
    /// Erreur du port runtime.
    #[error("apparatus apply: {0}")]
    Runtime(String),
}

struct DueRow {
    component_id: Uuid,
    digest: Option<String>,
    desired_generation: i64,
    component_type: String,
}

/// Applique une fois les bindings dûs (claim, bind, CAS observed).
///
/// # Errors
///
/// Retourne [`RuntimeApplyError`] si SQL ou le port échoue.
pub async fn apply_due_once(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    owner: &str,
    now: DateTime<Utc>,
) -> Result<(), RuntimeApplyError> {
    apply_due_once_inner(db, runtime, owner, now, true).await
}

/// Comme [`apply_due_once`] mais omet le CAS observed (reprise crash).
///
/// # Errors
///
/// Retourne [`RuntimeApplyError`] si SQL ou le port échoue.
pub async fn apply_due_once_skip_observe(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    owner: &str,
    now: DateTime<Utc>,
) -> Result<(), RuntimeApplyError> {
    apply_due_once_inner(db, runtime, owner, now, false).await
}

/// Bindings dûs + jobs de démontage dûs.
///
/// # Errors
///
/// Retourne [`RuntimeApplyError`] si SQL ou le port échoue.
pub async fn run_once(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    owner: &str,
    now: DateTime<Utc>,
) -> Result<(), RuntimeApplyError> {
    apply_due_once(db, runtime, owner, now).await?;
    apply_cleanup_due(db, runtime, now).await
}

async fn apply_due_once_inner(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    owner: &str,
    now: DateTime<Utc>,
    write_observe: bool,
) -> Result<(), RuntimeApplyError> {
    let due = select_due(db, now).await?;
    for row in due {
        if let Err(error) = apply_one(db, runtime, owner, now, &row, write_observe).await {
            tracing::warn!(
                %error,
                component_id = %row.component_id,
                "apparatus apply row failed"
            );
            continue;
        }
    }
    Ok(())
}

async fn select_due(db: &DatabaseConnection, now: DateTime<Utc>) -> Result<Vec<DueRow>, DbErr> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT b.component_id, b.digest, b.desired_generation, pc.component_type \
             FROM apparatus_bindings b \
             INNER JOIN project_components pc ON pc.id = b.component_id \
             WHERE b.source = 'managed' \
               AND b.desired_generation > b.observed_generation \
               AND (b.next_retry_at IS NULL OR b.next_retry_at <= $1) \
               AND NOT (b.next_retry_at IS NULL AND b.last_error_code IS NOT NULL)",
            [now.into()],
        ))
        .await?;
    let mut due = Vec::with_capacity(rows.len());
    for row in rows {
        due.push(DueRow {
            component_id: row.try_get("", "component_id")?,
            digest: row.try_get("", "digest")?,
            desired_generation: row.try_get("", "desired_generation")?,
            component_type: row.try_get("", "component_type")?,
        });
    }
    Ok(due)
}

async fn apply_one(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    owner: &str,
    now: DateTime<Utc>,
    row: &DueRow,
    write_observe: bool,
) -> Result<(), RuntimeApplyError> {
    let Some(digest_text) = row.digest.as_deref() else {
        return Ok(());
    };
    let Some(epoch) = claim_binding(db, row.component_id, owner, now).await? else {
        return Ok(());
    };
    let request = bind_request(row, digest_text)?;
    runtime
        .bind(&request)
        .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?;
    if write_observe {
        let rows_affected = write_observed(
            db,
            row.component_id,
            row.desired_generation,
            digest_text,
            epoch,
            owner,
            now,
        )
        .await?;
        if rows_affected == 0 {
            tracing::warn!(
                component_id = %row.component_id,
                generation = row.desired_generation,
                epoch,
                "apparatus fencing refused observed write"
            );
        }
    }
    Ok(())
}

/// Boucle d'intervalle : `apply_due_once` à chaque tick.
pub async fn run_tick_loop(
    db: DatabaseConnection,
    runtime: Arc<dyn ApparatusRuntime>,
    owner: String,
    tick_interval: Duration,
) {
    let start = tokio::time::Instant::now() + tick_interval;
    let mut interval = tokio::time::interval_at(start, tick_interval);
    loop {
        interval.tick().await;
        let now = Utc::now();
        if let Err(error) = run_once(&db, runtime.as_ref(), &owner, now).await {
            tracing::warn!(%error, "apparatus tick failed");
        }
    }
}

fn bind_request(row: &DueRow, digest_text: &str) -> Result<BindRequest, RuntimeApplyError> {
    let apparatus = legacy_to_apparatus(&row.component_type).ok_or_else(|| {
        RuntimeApplyError::Runtime(format!("unmapped component type {}", row.component_type))
    })?;
    let generation = u64::try_from(row.desired_generation)
        .map_err(|_| RuntimeApplyError::Runtime("desired_generation is negative".to_owned()))?;
    Ok(BindRequest {
        binding_id: BindingId::new(&row.component_id.to_string())
            .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?,
        apparatus_id: ApparatusId::new(apparatus)
            .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?,
        release_digest: ReleaseDigest::new(digest_text)
            .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?,
        operation_id: derived_operation_id(row.component_id, generation, "bind"),
        config: None,
    })
}
