//! Backfill Apparatus legacy — extension 1:1 `apparatus_bindings`.
//!
//! Remplit une ligne d'extension par `project_components` existant avec
//! `source = 'legacy'` et `digest = NULL` (non résolu).
//! Idempotent et rejouable : `ON CONFLICT (component_id) DO NOTHING`.
//! Ne modifie jamais `project_components` (lecture seule) et ne démarre
//! aucun traitement annexe : pilotage inactif, zéro charge applicative.
//!
//! Réversible : `DELETE FROM apparatus_bindings WHERE source = 'legacy'`
//! ou `down` T1 (retire la table). Aucun UUID public ajouté, aucun statut
//! produit, aucune écriture hors `apparatus_bindings`.

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

/// Insère les extensions legacy manquantes (`source = 'legacy'`, `digest = NULL`).
///
/// Parcourt `project_components` et crée la ligne `apparatus_bindings`
/// correspondante quand elle est absente. Rejouer deux fois produit le
/// même état (conflit `component_id` ignoré).
///
/// # Errors
///
/// Retourne [`DbErr`] si la connexion ou l'insertion échoue.
pub async fn backfill_apparatus_legacy<C>(db: &C) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    let stmt = Statement::from_string(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, digest, source) \
         SELECT id, NULL, 'legacy' FROM project_components \
         ON CONFLICT (component_id) DO NOTHING"
            .to_owned(),
    );
    let res = db.execute(stmt).await?;
    Ok(res.rows_affected())
}
