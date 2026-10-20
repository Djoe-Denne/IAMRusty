//! Jobs de démontage relançables (`apparatus_cleanup_jobs`).

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use uuid::Uuid;

use apparatus_contracts::{ApparatusRuntime, BindingId};

use super::tick::RuntimeApplyError;
use super::{next_backoff, APPARATUS_ERROR_TEARDOWN_FAILED};

struct DueJob {
    id: i64,
    project_id: Uuid,
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
            "SELECT id, project_id, component_id FROM apparatus_cleanup_jobs j \
             WHERE completed_at IS NULL \
               AND NOT EXISTS (SELECT 1 FROM apparatus_runtime_bindings r \
                   WHERE r.project_id=j.project_id AND r.binding_id=j.component_id) \
               AND (next_retry_at IS NULL OR next_retry_at <= $1) \
               AND NOT (next_retry_at IS NULL AND last_error_code IS NOT NULL)",
            [now.into()],
        ))
        .await?;
    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        jobs.push(DueJob {
            id: row.try_get("", "id")?,
            project_id: row.try_get("", "project_id")?,
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
    // Recheck after selection: a committed migration/tombstone permanently
    // fences this historical cleanup path. Never delete a Serving child.
    let migrated = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT 1 FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2",
            [job.project_id.into(), job.component_id.into()],
        ))
        .await?;
    if migrated.is_some() {
        return Ok(());
    }
    let binding = BindingId::new(&job.component_id.to_string())
        .map_err(|error| RuntimeApplyError::Runtime(error.to_string()))?;
    if let Err(error) = runtime.teardown(&binding) {
        tracing::warn!(
            %error,
            component_id = %job.component_id,
            "apparatus teardown failed"
        );
        let rows_affected = write_cleanup_failure(db, job.id, now).await?;
        if rows_affected == 0 {
            tracing::warn!(
                job_id = job.id,
                component_id = %job.component_id,
                "apparatus cleanup failure write skipped (already completed)"
            );
        }
        return Ok(());
    }
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

/// Retry / backoff / terminal du job. 0 ligne = déjà `completed_at` : pas d'incrément.
///
/// # Errors
///
/// Retourne [`DbErr`] si le SELECT ou l'UPDATE échoue.
async fn write_cleanup_failure(
    db: &DatabaseConnection,
    job_id: i64,
    now: DateTime<Utc>,
) -> Result<u64, DbErr> {
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT retry_count FROM apparatus_cleanup_jobs \
             WHERE id = $1 AND completed_at IS NULL",
            [job_id.into()],
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
            "UPDATE apparatus_cleanup_jobs \
             SET retry_count = $1, \
                 last_error_code = $2, \
                 next_retry_at = $3 \
             WHERE id = $4 AND completed_at IS NULL AND retry_count = $5",
            [
                new_count.into(),
                APPARATUS_ERROR_TEARDOWN_FAILED.into(),
                next_retry_at.into(),
                job_id.into(),
                current.into(),
            ],
        ))
        .await?;
    Ok(result.rows_affected())
}
