//! Jobs de démontage relançables (`apparatus_cleanup_jobs`).

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use uuid::Uuid;

use apparatus_contracts::{ApparatusRuntime, BindingId};

use super::tick::RuntimeApplyError;

struct DueJob {
    id: i64,
    component_id: Uuid,
}

/// Traite les jobs dûs : `teardown` puis CAS `completed_at`.
///
/// # Errors
///
/// Retourne [`RuntimeApplyError`] si SQL ou le port échoue.
pub async fn apply_cleanup_due(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    now: DateTime<Utc>,
) -> Result<(), RuntimeApplyError> {
    let jobs = select_due_jobs(db, now).await?;
    for job in jobs {
        if let Err(error) = complete_job(db, runtime, now, &job).await {
            tracing::warn!(
                %error,
                component_id = %job.component_id,
                "apparatus cleanup job failed"
            );
            continue;
        }
    }
    Ok(())
}

async fn select_due_jobs(
    db: &DatabaseConnection,
    now: DateTime<Utc>,
) -> Result<Vec<DueJob>, DbErr> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, component_id FROM apparatus_cleanup_jobs \
             WHERE completed_at IS NULL \
               AND (next_retry_at IS NULL OR next_retry_at <= $1)",
            [now.into()],
        ))
        .await?;
    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        jobs.push(DueJob {
            id: row.try_get("", "id")?,
            component_id: row.try_get("", "component_id")?,
        });
    }
    Ok(jobs)
}

async fn complete_job(
    db: &DatabaseConnection,
    runtime: &dyn ApparatusRuntime,
    now: DateTime<Utc>,
    job: &DueJob,
) -> Result<(), RuntimeApplyError> {
    let binding = BindingId::new(&job.component_id.to_string())
        .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?;
    runtime
        .teardown(&binding)
        .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?;
    let _ = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE apparatus_cleanup_jobs \
             SET completed_at = $1 \
             WHERE id = $2 AND completed_at IS NULL",
            [now.into(), job.id.into()],
        ))
        .await?;
    Ok(())
}
