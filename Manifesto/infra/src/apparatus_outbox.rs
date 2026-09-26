//! Écriture atomique du binding Apparatus + étape externe (T4 / T3).
//!
//! Insère la ligne `apparatus_bindings` (`source = 'managed'`,
//! `desired_generation = 1`, `next_retry_at = now()`, digest catalogue optionnel)
//! et exécute la fermeture externe fournie par l'appelant dans la
//! même transaction Postgres : tout échec (insertion ou fermeture) annule
//! l'ensemble, rien n'est partiellement persisté.
//!
//! Le job interne `apparatus_cleanup_jobs` suit le même schéma d'atomicité.
//! Aucune tâche de fond, aucun sondage, aucun démarrage annexe : la fermeture
//! est synchrone et fournie par l'appelant.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};
use std::fmt::Display;
use uuid::Uuid;

const INSERT_MANAGED_BINDING: &str = "INSERT INTO apparatus_bindings \
     (component_id, source, desired_generation, next_retry_at, digest, declared_capabilities) \
     VALUES ($1, 'managed', 1, NOW(), $2, $3::jsonb)";

const INSERT_CLEANUP_JOB: &str = "INSERT INTO apparatus_cleanup_jobs \
     (component_id, project_id, desired_generation, digest) \
     VALUES ($1, $2, $3, $4)";

const SELECT_BINDING_SOURCE_GENERATION: &str = "SELECT source, desired_generation \
     FROM apparatus_bindings WHERE component_id = $1";

const BUMP_MANAGED_DESIRED_GENERATION: &str = "UPDATE apparatus_bindings \
     SET desired_generation = desired_generation + 1, \
         next_retry_at = NOW(), \
         last_error_code = NULL, \
         retry_count = 0 \
     WHERE component_id = $1 AND source = 'managed' AND desired_generation = $2";

/// Échec de l'écriture atomique T4 (insertion annulée dans tous les cas).
#[derive(Debug, thiserror::Error)]
pub enum ApparatusAtomicError {
    /// Échec base de données (`BEGIN`, insertion ou `COMMIT`).
    #[error("apparatus atomic write failed (db): {0}")]
    Db(#[from] DbErr),
    /// Échec de la fermeture externe : l'insertion a été annulée.
    #[error("apparatus atomic write failed (external step): {0}")]
    External(String),
}

/// Ligne binding lue avant suppression du composant (CASCADE).
pub(crate) struct BindingCleanupRow {
    pub source: String,
    pub digest: Option<String>,
    pub desired_generation: i64,
}

/// Insère un binding `managed` avec `desired_generation = 1` et `next_retry_at = now()`.
///
/// `digest` / `declared_capabilities` viennent du catalogue à l'attache (ADR-0605).
/// Types non Apparatus : `digest = None` et `declared_capabilities = []`.
///
/// # Errors
///
/// Retourne [`DbErr`] si l'insertion échoue.
pub(crate) async fn insert_managed_binding<C>(
    db: &C,
    component_id: Uuid,
    digest: Option<String>,
    declared_capabilities: Vec<String>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let declared_json = serde_json::to_string(&declared_capabilities)
        .expect("declared_capabilities is a Vec<String>");
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        INSERT_MANAGED_BINDING,
        [component_id.into(), digest.into(), declared_json.into()],
    ))
    .await
    .map(|_| ())
}

/// Insère un job de cleanup (pas de FK ; survit au CASCADE du binding).
///
/// # Errors
///
/// Retourne [`DbErr`] si l'insertion échoue.
pub(crate) async fn insert_cleanup_job<C>(
    db: &C,
    component_id: Uuid,
    project_id: Uuid,
    generation: i64,
    digest: Option<String>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        INSERT_CLEANUP_JOB,
        [
            component_id.into(),
            project_id.into(),
            generation.into(),
            digest.into(),
        ],
    ))
    .await
    .map(|_| ())
}

/// Lit `source`, `digest`, `desired_generation` du binding, s'il existe.
///
/// # Errors
///
/// Retourne [`DbErr`] si la lecture échoue.
pub(crate) async fn load_binding_cleanup_row<C>(
    db: &C,
    component_id: Uuid,
) -> Result<Option<BindingCleanupRow>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT source, digest, desired_generation \
             FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(BindingCleanupRow {
        source: row.try_get("", "source")?,
        digest: row.try_get("", "digest")?,
        desired_generation: row.try_get("", "desired_generation")?,
    }))
}

/// Job requis si `managed` et (`digest` présent ou `desired_generation > 0`).
pub(crate) fn cleanup_job_required(row: &BindingCleanupRow) -> bool {
    row.source == "managed" && (row.digest.is_some() || row.desired_generation > 0)
}

enum BumpOutcome {
    Done,
    Conflict,
}

/// CAS `desired_generation + 1` pour un binding `managed`.
///
/// Pas de ligne ou `source != managed` : no-op. Un conflit concurrent
/// (0 ligne après un SELECT `managed`) est retenté une fois, puis [`DbErr`].
///
/// # Errors
///
/// Retourne [`DbErr`] si la lecture/écriture échoue, ou si le CAS échoue
/// deux fois de suite.
pub(crate) async fn bump_managed_desired_generation<C>(
    db: &C,
    component_id: Uuid,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    match try_bump_managed_desired_generation(db, component_id).await? {
        BumpOutcome::Done => Ok(()),
        BumpOutcome::Conflict => match try_bump_managed_desired_generation(db, component_id).await?
        {
            BumpOutcome::Done => Ok(()),
            BumpOutcome::Conflict => Err(DbErr::Custom(
                "apparatus desired_generation conflict".to_owned(),
            )),
        },
    }
}

async fn try_bump_managed_desired_generation<C>(
    db: &C,
    component_id: Uuid,
) -> Result<BumpOutcome, DbErr>
where
    C: ConnectionTrait,
{
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SELECT_BINDING_SOURCE_GENERATION,
            [component_id.into()],
        ))
        .await?
    else {
        return Ok(BumpOutcome::Done);
    };
    let source: String = row.try_get("", "source")?;
    if source != "managed" {
        return Ok(BumpOutcome::Done);
    }
    let expected: i64 = row.try_get("", "desired_generation")?;
    let updated = db
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            BUMP_MANAGED_DESIRED_GENERATION,
            [component_id.into(), expected.into()],
        ))
        .await?;
    if updated.rows_affected() == 0 {
        Ok(BumpOutcome::Conflict)
    } else {
        Ok(BumpOutcome::Done)
    }
}

async fn commit_or_rollback(
    txn: sea_orm::DatabaseTransaction,
    result: Result<(), ApparatusAtomicError>,
) -> Result<(), ApparatusAtomicError> {
    match result {
        Ok(()) => {
            txn.commit().await?;
            Ok(())
        }
        Err(error) => {
            if let Err(rollback_err) = txn.rollback().await {
                tracing::error!(
                    "apparatus atomic write: rollback failed after error: {rollback_err}"
                );
            }
            Err(error)
        }
    }
}

/// Persiste un binding `managed` et exécute `publish` atomiquement.
///
/// `BEGIN`, `INSERT INTO apparatus_bindings`, `publish()`, puis `COMMIT` :
/// si l'insertion ou `publish` échoue, la transaction est annulée et aucune
/// ligne n'est persistée. Succès = exactement une ligne `managed` commitée.
///
/// # Errors
///
/// Retourne [`ApparatusAtomicError::Db`] si `BEGIN`, l'insertion ou `COMMIT`
/// échoue, [`ApparatusAtomicError::External`] si `publish` échoue (avec
/// annulation de l'insertion dans les deux cas).
pub async fn persist_binding_atomically<F, E>(
    db: &DatabaseConnection,
    component_id: Uuid,
    publish: F,
) -> Result<(), ApparatusAtomicError>
where
    F: FnOnce() -> Result<(), E>,
    E: Display,
{
    let txn = db.begin().await?;
    let insert = insert_managed_binding(&txn, component_id, None, Vec::new()).await;
    let result: Result<(), ApparatusAtomicError> = match insert {
        Err(db_err) => Err(ApparatusAtomicError::Db(db_err)),
        Ok(()) => {
            publish().map_err(|publish_err| ApparatusAtomicError::External(publish_err.to_string()))
        }
    };
    commit_or_rollback(txn, result).await
}

/// Persiste un job de cleanup et exécute `publish` atomiquement.
///
/// # Errors
///
/// Retourne [`ApparatusAtomicError::Db`] si `BEGIN`, l'insertion ou `COMMIT`
/// échoue, [`ApparatusAtomicError::External`] si `publish` échoue (avec
/// annulation de l'insertion dans les deux cas).
pub async fn persist_cleanup_job_atomically<F, E>(
    db: &DatabaseConnection,
    component_id: Uuid,
    project_id: Uuid,
    generation: i64,
    digest: Option<String>,
    publish: F,
) -> Result<(), ApparatusAtomicError>
where
    F: FnOnce() -> Result<(), E>,
    E: Display,
{
    let txn = db.begin().await?;
    let insert = insert_cleanup_job(&txn, component_id, project_id, generation, digest).await;
    let result: Result<(), ApparatusAtomicError> = match insert {
        Err(db_err) => Err(ApparatusAtomicError::Db(db_err)),
        Ok(()) => {
            publish().map_err(|publish_err| ApparatusAtomicError::External(publish_err.to_string()))
        }
    };
    commit_or_rollback(txn, result).await
}
