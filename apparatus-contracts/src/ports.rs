//! Ports hexagonaux Apparatus P0 : abstractions fournies par l'hôte.
//!
//! Le domaine (`apparatus-contracts`) définit les ports ; les adaptateurs
//! (`apparatus-reference-kv`, harness TEST-ONLY) les implémentent. Aucune E/S
//! ici : uniquement des traits synchrones et purs.

use crate::error::ApparatusError;
use crate::ids::BindingId;

/// Stockage KV abstrait `kv-v1`, fourni par l'hôte (harness en P0).
///
/// Toutes les méthodes sont synchrones et pures (ni réseau, ni SQL, ni E/S).
/// L'implémentation namespacée par `binding_id` est exigée : l'appelant ne
/// fournit que `key`, l'adaptateur impose l'isolation.
pub trait KvStore: Send + Sync {
    /// Lit une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est hors bornes.
    fn kv_get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError>;

    /// Écrit une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] ou
    /// [`ApparatusError::PayloadTooLarge`] hors bornes.
    fn kv_put(&self, binding: &BindingId, key: &str, value: &[u8]) -> Result<(), ApparatusError>;

    /// Supprime une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est hors bornes.
    fn kv_delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError>;

    /// Purge le namespace d'un binding.
    fn kv_purge(&self, binding: &BindingId);
}
