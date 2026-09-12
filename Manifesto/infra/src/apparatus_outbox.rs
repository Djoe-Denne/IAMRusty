//! Écriture atomique du binding Apparatus + étape externe (T4).
//!
//! Insère la ligne `apparatus_bindings` (`source = 'managed'`, `digest = NULL`
//! non résolu) et exécute la fermeture externe fournie par l'appelant dans la
//! même transaction Postgres : tout échec (insertion ou fermeture) annule
//! l'ensemble, rien n'est partiellement persisté.
//!
//! Aucune tâche de fond, aucun sondage, aucun démarrage annexe : la fermeture
//! est synchrone et fournie par l'appelant (les tests injectent le faux
//! courtier in-memory). Aucun consentement ni génération ajoutés ici : renvoyés
//! à une ADR dédiée avant P2. Réutilise la table T1, additive et réversible.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};
use std::fmt::Display;
use uuid::Uuid;

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
    let insert = txn
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO apparatus_bindings (component_id, source) VALUES ($1, 'managed')",
            [component_id.into()],
        ))
        .await;
    let result: Result<(), ApparatusAtomicError> = match insert {
        Err(db_err) => Err(ApparatusAtomicError::Db(db_err)),
        Ok(_) => {
            publish().map_err(|publish_err| ApparatusAtomicError::External(publish_err.to_string()))
        }
    };
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
