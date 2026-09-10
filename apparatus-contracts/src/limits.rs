//! Bornes explicites des contrats Apparatus P0.
//!
//! Toutes les limites sont centralisées ici pour rester auditables et stables.
//! Les validateurs et les DTO refusent toute valeur hors bornes avec une erreur
//! versionnée, sans jamais renvoyer le contenu sensible.

/// Longueur maximale d'un identifiant (`apparatus_id`, `binding_id`, nom d'opération).
///
/// Cohérente avec la limite actuelle de 100 caractères de `component_type`.
pub const MAX_ID_LEN: usize = 100;

/// Longueur maximale d'une clé KV (en caractères).
pub const MAX_KV_KEY_LEN: usize = 256;

/// Taille maximale d'une valeur KV (en octets).
pub const MAX_KV_VALUE_BYTES: usize = 64 * 1024;

/// Taille maximale d'un document de configuration sérialisé (en octets, JSON compact).
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;

/// Taille maximale d'un payload d'invocation sérialisé (en octets, JSON compact).
///
/// Couvre `params` et `result` du protocole `manifesto-apparatus/1`.
pub const MAX_PAYLOAD_BYTES: usize = 256 * 1024;

/// Nombre maximal de capacités déclarées par un manifeste.
pub const MAX_CAPABILITIES: usize = 32;

/// Longueur maximale d'un nom d'opération `invoke` (en caractères).
pub const MAX_OPERATION_NAME_LEN: usize = 100;

/// Longueur maximale d'un chemin de schéma UI déclaré (en caractères).
pub const MAX_SCHEMA_PATH_LEN: usize = 256;

/// Longueur maximale d'une référence d'installation brute avant validation.
pub const MAX_INSTALL_REF_LEN: usize = 256;

/// Longueur maximale de la description du manifeste (en caractères).
pub const MAX_DESCRIPTION_LEN: usize = 1024;
