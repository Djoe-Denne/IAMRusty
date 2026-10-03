//! Protocole backend privé versionné `manifesto-apparatus/1`.
//!
//! Découverte via `/.well-known/apparatus`, sondes `health`/`ready`, puis
//! cycle `bind` → `configure` → `invoke` → `unbind`. Tous les DTO entrants
//! appliquent `deny_unknown_fields`. Chaque requête de cycle porte un
//! [`OperationId`] qui rend l'opération idempotente
//! par rejeu : même `operation_id` ⇒ même réponse, sans nouvel effet.
//!
//! Aucun DTO ne transporte de secret, bearer, JWT ou HMAC (ADR-0004).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::ApparatusError;
use crate::ids::{ApparatusId, BindingId, OperationId, ReleaseDigest};
use crate::limits::{MAX_CONFIG_BYTES, MAX_ID_LEN, MAX_OPERATION_NAME_LEN, MAX_PAYLOAD_BYTES};

/// Identifiant versionné du protocole wire P0.
pub const PROTOCOL_ID: &str = "manifesto-apparatus/1";

/// Chemin de découverte du backend.
pub const WELL_KNOWN_PATH: &str = "/.well-known/apparatus";

/// Chemin de la sonde de santé.
pub const HEALTH_PATH: &str = "/health";

/// Chemin de la sonde de readiness.
pub const READY_PATH: &str = "/ready";

/// Chemin de l'opération `bind`.
pub const BIND_PATH: &str = "/bind";

/// Chemin de l'opération `configure`.
pub const CONFIGURE_PATH: &str = "/configure";

/// Chemin de l'opération `invoke`.
pub const INVOKE_PATH: &str = "/invoke";

/// Chemin de l'opération `unbind`.
pub const UNBIND_PATH: &str = "/unbind";

/// Génère un nouvel identifiant d'opération (UUID v4).
#[must_use]
pub fn new_operation_id() -> OperationId {
    OperationId::new()
}

/// Document de découverte servi sur `/.well-known/apparatus`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryDocument {
    /// Doit valoir [`PROTOCOL_ID`].
    pub protocol: String,
    /// Identité de l'Apparatus servi.
    pub apparatus_id: ApparatusId,
    /// Version déclarative `SemVer` de la release servie.
    pub version: String,
    /// Digest immuable de la release servie.
    pub release_digest: ReleaseDigest,
    /// Chemins des opérations.
    pub endpoints: DiscoveryEndpoints,
}

/// Chemins des opérations du backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryEndpoints {
    /// Chemin `health`.
    pub health: String,
    /// Chemin `ready`.
    pub ready: String,
    /// Chemin `bind`.
    pub bind: String,
    /// Chemin `configure`.
    pub configure: String,
    /// Chemin `invoke`.
    pub invoke: String,
    /// Chemin `unbind`.
    pub unbind: String,
}

impl Default for DiscoveryEndpoints {
    /// Chemins canoniques du protocole P0.
    fn default() -> Self {
        Self {
            health: HEALTH_PATH.to_owned(),
            ready: READY_PATH.to_owned(),
            bind: BIND_PATH.to_owned(),
            configure: CONFIGURE_PATH.to_owned(),
            invoke: INVOKE_PATH.to_owned(),
            unbind: UNBIND_PATH.to_owned(),
        }
    }
}

/// État de santé du backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HealthStatus {
    /// Backend sain.
    Ok,
    /// Backend dégradé (reste joignable).
    Degraded,
}

/// Réponse de la sonde `health` (vivacité, sans détail sensible).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthResponse {
    /// État de santé.
    pub status: HealthStatus,
    /// Identité de l'Apparatus sondé.
    pub apparatus_id: ApparatusId,
}

/// Réponse de la sonde `ready` (prêt à servir le cycle bind/invoke).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyResponse {
    /// `true` si le backend accepte le cycle de vie.
    pub ready: bool,
    /// Identité de l'Apparatus sondé.
    pub apparatus_id: ApparatusId,
    /// Digest de la release prête.
    pub release_digest: ReleaseDigest,
}

/// Requête `bind` : attache un `binding_id` à une release admise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindRequest {
    /// Binding à créer (idempotent par `operation_id`).
    pub binding_id: BindingId,
    /// Apparatus attendu.
    pub apparatus_id: ApparatusId,
    /// Digest de release attendu.
    pub release_digest: ReleaseDigest,
    /// Clé d'idempotence.
    pub operation_id: OperationId,
    /// Configuration initiale optionnelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, serde_json::Value>>,
}

/// Réponse `bind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindResponse {
    /// Binding attaché.
    pub binding_id: BindingId,
    /// Clé d'idempotence rejouée.
    pub operation_id: OperationId,
    /// `true` si l'effet a été appliqué (ou déjà appliqué au rejeu).
    pub applied: bool,
}

/// Requête `configure` : remplace la configuration d'un binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureRequest {
    /// Binding à configurer.
    pub binding_id: BindingId,
    /// Clé d'idempotence.
    pub operation_id: OperationId,
    /// Configuration complète (bornée par [`MAX_CONFIG_BYTES`]).
    pub config: BTreeMap<String, serde_json::Value>,
}

/// Réponse `configure`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureResponse {
    /// Binding configuré.
    pub binding_id: BindingId,
    /// Clé d'idempotence rejouée.
    pub operation_id: OperationId,
    /// `true` si l'effet a été appliqué (ou déjà appliqué au rejeu).
    pub applied: bool,
}

/// Requête `invoke` : exécute une opération nommée sur un binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeRequest {
    /// Binding cible.
    pub binding_id: BindingId,
    /// Clé d'idempotence.
    pub operation_id: OperationId,
    /// Nom d'opération (ex. `kv.get`), borné par [`MAX_OPERATION_NAME_LEN`].
    pub operation: String,
    /// Paramètres (bornés par [`MAX_PAYLOAD_BYTES`] sérialisés).
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Réponse `invoke`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeResponse {
    /// Binding cible.
    pub binding_id: BindingId,
    /// Clé d'idempotence rejouée.
    pub operation_id: OperationId,
    /// Résultat (borné par [`MAX_PAYLOAD_BYTES`] sérialisé).
    pub result: serde_json::Value,
}

/// Requête `unbind` : détache un binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnbindRequest {
    /// Binding à détacher.
    pub binding_id: BindingId,
    /// Clé d'idempotence.
    pub operation_id: OperationId,
}

/// Réponse `unbind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnbindResponse {
    /// Binding détaché.
    pub binding_id: BindingId,
    /// Clé d'idempotence rejouée.
    pub operation_id: OperationId,
    /// `true` si l'effet a été appliqué (ou déjà appliqué au rejeu).
    pub applied: bool,
}

/// Taille JSON compacte d'une valeur, en octets.
///
/// Retourne une borne dépassée plutôt qu'une erreur de sérialisation :
/// nos types sont sérialisables en pratique.
fn compact_len(value: &serde_json::Value) -> usize {
    serde_json::to_string(value).map_or(MAX_PAYLOAD_BYTES + 1, |text| text.len())
}

impl BindRequest {
    /// Valide les bornes de la requête (config incluse).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::ConfigTooLarge`] si la configuration
    /// sérialisée dépasse [`MAX_CONFIG_BYTES`].
    pub fn validate(&self) -> Result<(), ApparatusError> {
        if let Some(config) = &self.config {
            let value = serde_json::to_value(config).map_err(|err| ApparatusError::Json {
                message: ApparatusError::truncate(&err.to_string(), 200),
            })?;
            let actual = compact_len(&value);
            if actual > MAX_CONFIG_BYTES {
                return Err(ApparatusError::ConfigTooLarge {
                    max: MAX_CONFIG_BYTES,
                    actual,
                });
            }
        }
        Ok(())
    }
}

impl ConfigureRequest {
    /// Valide les bornes de la requête.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::ConfigTooLarge`] si la configuration
    /// sérialisée dépasse [`MAX_CONFIG_BYTES`], [`ApparatusError::Json`]
    /// si la conversion échoue.
    pub fn validate(&self) -> Result<(), ApparatusError> {
        let value = serde_json::to_value(&self.config).map_err(|err| ApparatusError::Json {
            message: ApparatusError::truncate(&err.to_string(), 200),
        })?;
        let actual = compact_len(&value);
        if actual > MAX_CONFIG_BYTES {
            return Err(ApparatusError::ConfigTooLarge {
                max: MAX_CONFIG_BYTES,
                actual,
            });
        }
        Ok(())
    }
}

impl InvokeRequest {
    /// Valide les bornes de la requête (nom + paramètres).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si le nom est vide ou
    /// dépasse [`MAX_OPERATION_NAME_LEN`], [`ApparatusError::PayloadTooLarge`]
    /// si les paramètres sérialisés dépassent [`MAX_PAYLOAD_BYTES`].
    pub fn validate(&self) -> Result<(), ApparatusError> {
        let name_len = self.operation.chars().count();
        if self.operation.is_empty() || name_len > MAX_OPERATION_NAME_LEN {
            return Err(ApparatusError::InvalidOperation {
                reason: "operation name length out of bounds".to_owned(),
            });
        }
        let actual = compact_len(&self.params);
        if actual > MAX_PAYLOAD_BYTES {
            return Err(ApparatusError::PayloadTooLarge {
                max: MAX_PAYLOAD_BYTES,
                actual,
            });
        }
        Ok(())
    }
}

impl InvokeResponse {
    /// Valide les bornes de la réponse (résultat).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::PayloadTooLarge`] si le résultat sérialisé
    /// dépasse [`MAX_PAYLOAD_BYTES`].
    pub fn validate(&self) -> Result<(), ApparatusError> {
        let actual = compact_len(&self.result);
        if actual > MAX_PAYLOAD_BYTES {
            return Err(ApparatusError::PayloadTooLarge {
                max: MAX_PAYLOAD_BYTES,
                actual,
            });
        }
        Ok(())
    }
}

impl UnbindRequest {
    /// Valide la requête `unbind`.
    ///
    /// Les identifiants sont déjà validés à la frontière wire (`FromStr`) ;
    /// cette méthode existe pour la cohérence d'API du cycle de vie
    /// (`bind`/`configure`/`invoke`/`unbind` valident tous leur requête).
    ///
    /// # Errors
    ///
    /// Ne retourne jamais d'erreur en P0 (signature en `Result` pour stabilité).
    pub const fn validate(&self) -> Result<(), ApparatusError> {
        // `self` est ignoré : validation triviale, cohérence d'API.
        let _ = self;
        Ok(())
    }
}

/// Valide qu'une chaîne libre tient dans [`MAX_ID_LEN`] caractères.
///
/// Utilisé pour les champs de détail itérables (noms, chemins courts).
///
/// # Errors
///
/// Retourne [`ApparatusError::IdTooLong`] si la valeur dépasse la borne.
pub fn validate_short_text(value: &str) -> Result<(), ApparatusError> {
    let actual = value.chars().count();
    if actual > MAX_ID_LEN {
        return Err(ApparatusError::IdTooLong {
            max: MAX_ID_LEN,
            actual,
        });
    }
    Ok(())
}
