//! Écriture atomique du binding Apparatus + étape externe (T4 / T3).
//!
//! Insère la ligne `apparatus_bindings` (`source = 'managed'`,
//! `desired_generation = 1`, `next_retry_at = now()`, `digest` non résolu)
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
     (component_id, source, desired_generation, next_retry_at) \
     VALUES ($1, 'managed', 1, NOW())";

const INSERT_CLEANUP_JOB: &str = "INSERT INTO apparatus_cleanup_jobs \
     (component_id, project_id, desired_generation, digest) \
     VALUES ($1, $2, $3, $4)";

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
/// # Errors
///
/// Retourne [`DbErr`] si l'insertion échoue.
pub(crate) async fn insert_managed_binding<C>(db: &C, component_id: Uuid) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        INSERT_MANAGED_BINDING,
        [component_id.into()],
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
    let insert = insert_managed_binding(&txn, component_id).await;
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
