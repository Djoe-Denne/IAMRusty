//! Ports hexagonaux Apparatus P0 : abstractions fournies par l'hôte.
//!
//! Le domaine (`apparatus-contracts`) définit les ports ; les adaptateurs
//! (`apparatus-reference-kv`, harness TEST-ONLY) les implémentent. Aucune E/S
//! ici : uniquement des traits synchrones et purs.

use crate::error::ApparatusError;
use crate::ids::BindingId;
use crate::protocol::{
    BindRequest, BindResponse, ConfigureRequest, ConfigureResponse, UnbindRequest, UnbindResponse,
};

/// Stockage KV abstrait `kv-v1`, fourni par l'hôte.
///
/// L'implémentation namespacée par `binding_id` est exigée : l'appelant ne
/// fournit que `key`, l'adaptateur impose l'isolation. Le harness P0 reste
/// in-memory ; les adaptateurs plateforme (Postgres, Redis) réalisent l'I/O.
pub trait KvStore: Send + Sync {
    /// Lit une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est hors bornes.
    fn kv_get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError>;

    /// Écrit une valeur pour un binding.
    ///
    /// `expected_cas` : `None` écrit inconditionnellement ; `Some(n)` exige
    /// que la version courante soit `n` (`0` si la clé est absente).
    /// Retourne la nouvelle `cas_version`.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] (clé, CAS, quota) ou
    /// [`ApparatusError::PayloadTooLarge`] hors bornes.
    fn kv_put(
        &self,
        binding: &BindingId,
        key: &str,
        value: &[u8],
        expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError>;

    /// Supprime une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est hors bornes.
    fn kv_delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError>;

    /// Purge le namespace d'un binding.
    fn kv_purge(&self, binding: &BindingId);
}

/// Observation déterministe d'un binding (digest appliqué + génération).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeObservation {
    /// Digest observé, s'il y a une instance attachée.
    pub digest: Option<String>,
    /// Génération observée (0 = jamais appliqué).
    pub generation: u64,
}

/// Port sync du cycle de vie Apparatus (comme [`KvStore`] : pas de Tokio).
///
/// Pas d'`invoke`. Pas d'`ensure_instance` / `release`.
pub trait ApparatusRuntime: Send + Sync {
    /// Attache un binding (idempotent par `operation_id`).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError`] si la requête est hors bornes.
    fn bind(&self, req: &BindRequest) -> Result<BindResponse, ApparatusError>;

    /// Remplace la configuration d'un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError`] si le binding est inconnu ou hors bornes.
    fn configure(&self, req: &ConfigureRequest) -> Result<ConfigureResponse, ApparatusError>;

    /// Détache un binding (idempotent par `operation_id`).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError`] si le binding est inconnu hors rejeu.
    fn unbind(&self, req: &UnbindRequest) -> Result<UnbindResponse, ApparatusError>;

    /// Observe l'instance attachée.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError`] si l'observation est impossible.
    fn observe(&self, binding: &BindingId) -> Result<RuntimeObservation, ApparatusError>;

    /// Démontage idempotent (pas de `release`).
    ///
    /// # Errors
    ///
    /// Réservé aux adaptateurs qui ne peuvent pas garantir l'idempotence.
    fn teardown(&self, binding: &BindingId) -> Result<(), ApparatusError>;
}
