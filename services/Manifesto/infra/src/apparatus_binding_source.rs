//! Lecture de la source d'un binding Apparatus (`legacy` | `managed`).
//!
//! Port d'infrastructure : une ligne absente de `apparatus_bindings` signifie
//! « appliquer le chemin existant » (composants non backfillés). Une source
//! inconnue n'est jamais traitée comme un succès silencieux.

use async_trait::async_trait;
use rustycog::core::error::ServiceError;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

/// Source persistée dans `apparatus_bindings.source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApparatusBindingSource {
    /// Binding issu du chemin existant (`project_id` + `component_type`).
    Legacy,
    /// Binding géré : un événement sans identité de binding ne doit pas l'altérer.
    Managed,
}

/// Résout la source d'un binding à partir de l'identité du composant.
#[async_trait]
pub trait ApparatusBindingSourceLookup: Send + Sync {
    /// Retourne la source liée à `component_id`, ou `None` si aucune ligne.
    ///
    /// # Errors
    ///
    /// Retourne [`ServiceError`] si la lecture échoue, ou si `source` n'est
    /// ni `legacy` ni `managed`.
    async fn source_for_component(
        &self,
        component_id: Uuid,
    ) -> Result<Option<ApparatusBindingSource>, ServiceError>;
}

/// Lookup SQL de `apparatus_bindings.source` par `component_id`.
pub struct SqlApparatusBindingSourceLookup {
    db: DatabaseConnection,
}

impl SqlApparatusBindingSourceLookup {
    /// Construit le lookup sur une connexion en lecture.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ApparatusBindingSourceLookup for SqlApparatusBindingSourceLookup {
    async fn source_for_component(
        &self,
        component_id: Uuid,
    ) -> Result<Option<ApparatusBindingSource>, ServiceError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT source FROM apparatus_bindings WHERE component_id = $1",
                [component_id.into()],
            ))
            .await
            .map_err(|error| {
                ServiceError::infrastructure(format!(
                    "failed to read apparatus binding source for {component_id}: {error}"
                ))
            })?;

        let Some(row) = row else {
            return Ok(None);
        };

        let source: String = row.try_get("", "source").map_err(|error| {
            ServiceError::infrastructure(format!(
                "failed to decode apparatus binding source for {component_id}: {error}"
            ))
        })?;

        match source.as_str() {
            "legacy" => Ok(Some(ApparatusBindingSource::Legacy)),
            "managed" => Ok(Some(ApparatusBindingSource::Managed)),
            other => Err(ServiceError::validation(format!(
                "unknown apparatus binding source '{other}' for component {component_id}"
            ))),
        }
    }
}
