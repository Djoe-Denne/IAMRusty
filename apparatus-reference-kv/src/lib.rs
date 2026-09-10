//! Apparatus KV de référence P0 : backend `kv-v1` injecté, UI `schema`.
//!
//! `apparatus_id` concret : [`REFERENCE_APPARATUS_ID`]. Aucune capacité réseau,
//! aucune lecture d'environnement sensible, handlers `bind`/`configure`/
//! `invoke`/`unbind` idempotents par [`OperationId`].
//! Le stockage [`KvStore`] est injecté ; l'isolation par `binding_id` est
//! imposée par l'adaptateur namespacé, jamais par un préfixe choisi par l'appelant.
//!
//! ## Tests
//!
//! ```sh
//! cargo test -p apparatus-reference-kv
//! ```

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, PoisonError};

use apparatus_contracts::{
    ApparatusError, BindRequest, BindResponse, BindingId, ConfigureRequest, ConfigureResponse,
    InvokeRequest, InvokeResponse, OperationId, UnbindRequest, UnbindResponse,
};
use thiserror::Error;

/// Port KV réexporté depuis les contrats (hexagone : port en domaine).
pub use apparatus_contracts::KvStore;

/// Identité concrète de l'Apparatus de référence (choix d'implémentation P0).
pub const REFERENCE_APPARATUS_ID: &str = "io.aiforall.reference-kv";

/// Version déclarative de la référence (`SemVer`, fixture `apparatus.toml`).
pub const REFERENCE_VERSION: &str = "0.1.0";

/// Adaptateur de stockage déclaré par la référence.
pub const REFERENCE_STORAGE: &str = "kv-v1";

/// Erreur du backend de référence (enveloppe fine au-dessus des contrats).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ReferenceKvError {
    /// Erreur de contrat sous-jacente (bornes, opération, binding).
    #[error("contract error: {0}")]
    Contract(#[from] ApparatusError),
}

/// Backend KV de référence, paramétré par un [`KvStore`] injecté.
#[derive(Debug)]
pub struct ReferenceKvBackend<S: KvStore> {
    /// Stockage injecté.
    store: S,
    /// Bindings attachés.
    bindings: Mutex<HashMap<String, BTreeMap<String, serde_json::Value>>>,
    /// Rejeu `bind` par `operation_id`.
    replay_bind: Mutex<HashMap<String, BindResponse>>,
    /// Rejeu `configure` par `operation_id`.
    replay_configure: Mutex<HashMap<String, ConfigureResponse>>,
    /// Rejeu `invoke` par `operation_id`.
    replay_invoke: Mutex<HashMap<String, InvokeResponse>>,
    /// Rejeu `unbind` par `operation_id`.
    replay_unbind: Mutex<HashMap<String, UnbindResponse>>,
}

impl<S: KvStore> ReferenceKvBackend<S> {
    /// Construit le backend autour d'un stockage injecté.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            bindings: Mutex::new(HashMap::new()),
            replay_bind: Mutex::new(HashMap::new()),
            replay_configure: Mutex::new(HashMap::new()),
            replay_invoke: Mutex::new(HashMap::new()),
            replay_unbind: Mutex::new(HashMap::new()),
        }
    }

    /// Indique si un binding est attaché.
    #[must_use]
    pub fn is_bound(&self, binding: &BindingId) -> bool {
        let guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
        guard.contains_key(binding.as_str())
    }

    /// Exécute `bind` (idempotent par `operation_id`).
    ///
    /// # Errors
    ///
    /// Retourne [`ReferenceKvError::Contract`] si la requête dépasse les bornes.
    pub fn bind(&self, request: &BindRequest) -> Result<BindResponse, ReferenceKvError> {
        request.validate()?;
        let key = request.operation_id.to_string();
        if let Some(stored) = replay_get(&self.replay_bind, &key) {
            return Ok(stored);
        }
        {
            let mut guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
            guard
                .entry(request.binding_id.as_str().to_owned())
                .or_default();
            if let Some(config) = &request.config {
                if let Some(entry) = guard.get_mut(request.binding_id.as_str()) {
                    *entry = config.clone();
                }
            }
        }
        let response = BindResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            applied: true,
        };
        replay_put(&self.replay_bind, &key, &response);
        Ok(response)
    }

    /// Exécute `configure` (idempotent par `operation_id`).
    ///
    /// # Errors
    ///
    /// Retourne [`ReferenceKvError::Contract`] si le binding est inconnu ou
    /// si la configuration dépasse les bornes.
    pub fn configure(
        &self,
        request: &ConfigureRequest,
    ) -> Result<ConfigureResponse, ReferenceKvError> {
        request.validate()?;
        let key = request.operation_id.to_string();
        if let Some(stored) = replay_get(&self.replay_configure, &key) {
            return Ok(stored);
        }
        self.require_bound(&request.binding_id)?;
        {
            let mut guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(entry) = guard.get_mut(request.binding_id.as_str()) {
                *entry = request.config.clone();
            }
        }
        let response = ConfigureResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            applied: true,
        };
        replay_put(&self.replay_configure, &key, &response);
        Ok(response)
    }

    /// Exécute `invoke` (idempotent par `operation_id`).
    ///
    /// Opérations : `kv.get`, `kv.put`, `kv.delete`.
    ///
    /// # Errors
    ///
    /// Retourne [`ReferenceKvError::Contract`] si le binding est inconnu, si
    /// l'opération ou les paramètres sont invalides, ou hors bornes.
    pub fn invoke(&self, request: &InvokeRequest) -> Result<InvokeResponse, ReferenceKvError> {
        request.validate()?;
        let key = request.operation_id.to_string();
        if let Some(stored) = replay_get(&self.replay_invoke, &key) {
            return Ok(stored);
        }
        self.require_bound(&request.binding_id)?;
        let result = dispatch(&self.store, &request.binding_id, request)?;
        let response = InvokeResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            result,
        };
        response.validate()?;
        replay_put(&self.replay_invoke, &key, &response);
        Ok(response)
    }

    /// Exécute `unbind` (idempotent par `operation_id`, purge le namespace).
    ///
    /// Le rejeu du même `operation_id` rejoue la réponse stockée même si le
    /// binding a déjà été purgé ; un nouvel `operation_id` sur un binding
    /// inconnu est rejeté.
    ///
    /// # Errors
    ///
    /// Retourne [`ReferenceKvError::Contract`] avec
    /// [`ApparatusError::InvalidOperation`] si le binding est inconnu
    /// (hors rejeu du même `operation_id`).
    pub fn unbind(&self, request: &UnbindRequest) -> Result<UnbindResponse, ReferenceKvError> {
        request.validate()?;
        let key = request.operation_id.to_string();
        if let Some(stored) = replay_get(&self.replay_unbind, &key) {
            return Ok(stored);
        }
        self.require_bound(&request.binding_id)?;
        {
            let mut guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
            guard.remove(request.binding_id.as_str());
        }
        self.store.kv_purge(&request.binding_id);
        let response = UnbindResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            applied: true,
        };
        replay_put(&self.replay_unbind, &key, &response);
        Ok(response)
    }

    /// Exige un binding attaché.
    fn require_bound(&self, binding: &BindingId) -> Result<(), ReferenceKvError> {
        if self.is_bound(binding) {
            Ok(())
        } else {
            Err(ApparatusError::InvalidOperation {
                reason: "unknown binding".to_owned(),
            }
            .into())
        }
    }
}

/// Lit une réponse rejouée.
fn replay_get<T: Clone>(slot: &Mutex<HashMap<String, T>>, key: &str) -> Option<T> {
    let guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard.get(key).cloned()
}

/// Stocke une réponse rejouable.
fn replay_put<T: Clone>(slot: &Mutex<HashMap<String, T>>, key: &str, value: &T) {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard.insert(key.to_owned(), value.clone());
}

/// Dispatche `kv.get` / `kv.put` / `kv.delete` vers le [`KvStore`] injecté.
fn dispatch<S: KvStore>(
    store: &S,
    binding: &BindingId,
    request: &InvokeRequest,
) -> Result<serde_json::Value, ApparatusError> {
    match request.operation.as_str() {
        "kv.get" => {
            let key = param_key(&request.params)?;
            store.kv_get(binding, key)?.map_or_else(
                || Ok(serde_json::json!({"found": false})),
                |bytes| {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    Ok(serde_json::json!({"found": true, "value": text}))
                },
            )
        }
        "kv.put" => {
            let key = param_key(&request.params)?;
            let value = param_value(&request.params)?;
            store.kv_put(binding, key, value.as_bytes())?;
            Ok(serde_json::json!({"stored": true}))
        }
        "kv.delete" => {
            let key = param_key(&request.params)?;
            let removed = store.kv_delete(binding, key)?;
            Ok(serde_json::json!({"removed": removed}))
        }
        _ => Err(ApparatusError::InvalidOperation {
            reason: "unknown reference operation".to_owned(),
        }),
    }
}

/// Extrait `key` des paramètres.
fn param_key(params: &serde_json::Value) -> Result<&str, ApparatusError> {
    params
        .get("key")
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || {
                Err(ApparatusError::InvalidOperation {
                    reason: "invoke params must carry key".to_owned(),
                })
            },
            Ok,
        )
}

/// Extrait `value` des paramètres.
fn param_value(params: &serde_json::Value) -> Result<&str, ApparatusError> {
    params
        .get("value")
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || {
                Err(ApparatusError::InvalidOperation {
                    reason: "kv.put params must carry value".to_owned(),
                })
            },
            Ok,
        )
}

/// Stockage en mémoire namespacé par `binding_id` (doublure de test).
///
/// L'isolation est imposée par la clé interne `(binding_id, key)`.
#[derive(Debug, Default)]
pub struct InMemoryKvStore {
    /// Entrées `(binding_id, key) -> valeur brute`.
    entries: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl InMemoryKvStore {
    /// Crée un stockage vide.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl KvStore for InMemoryKvStore {
    fn kv_get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError> {
        check_key(key)?;
        let guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(guard
            .get(&(binding.as_str().to_owned(), key.to_owned()))
            .cloned())
    }

    fn kv_put(&self, binding: &BindingId, key: &str, value: &[u8]) -> Result<(), ApparatusError> {
        check_key(key)?;
        if value.len() > apparatus_contracts::MAX_KV_VALUE_BYTES {
            return Err(ApparatusError::PayloadTooLarge {
                max: apparatus_contracts::MAX_KV_VALUE_BYTES,
                actual: value.len(),
            });
        }
        {
            let mut guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
            guard.insert(
                (binding.as_str().to_owned(), key.to_owned()),
                value.to_vec(),
            );
        }
        Ok(())
    }

    fn kv_delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError> {
        check_key(key)?;
        let mut guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(guard
            .remove(&(binding.as_str().to_owned(), key.to_owned()))
            .is_some())
    }

    fn kv_purge(&self, binding: &BindingId) {
        let mut guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        guard.retain(|(owner, _), _| owner != binding.as_str());
    }
}

/// Valide une clé KV : non vide, bornée.
fn check_key(key: &str) -> Result<(), ApparatusError> {
    if key.is_empty() || key.chars().count() > apparatus_contracts::MAX_KV_KEY_LEN {
        return Err(ApparatusError::InvalidOperation {
            reason: "kv key length out of bounds".to_owned(),
        });
    }
    Ok(())
}

/// Construit un `operation_id` déterministe pour les tests.
///
/// Dérive 16 octets via FNV-1a 64 bits (deux graines) du libellé puis
/// construit un UUID via [`OperationId::from_bytes`] : même libellé ⇒ même
/// `operation_id`, sans aléas, sans RNG, sans fallback non déterministe.
#[must_use]
pub fn deterministic_operation_id(label: &str) -> OperationId {
    let high = fnv1a64(label.as_bytes(), 0xcbf2_9ce4_8422_2325);
    let low = fnv1a64(label.as_bytes(), 0x8422_2325_cbf2_9ce4);
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&high.to_be_bytes());
    bytes[8..].copy_from_slice(&low.to_be_bytes());
    OperationId::from_bytes(bytes)
}

/// FNV-1a 64 bits avec graine explicite (déterministe, sans dépendance).
fn fnv1a64(input: &[u8], seed: u64) -> u64 {
    const PRIME: u64 = 0x100_0000_01b3;
    let mut hash = seed;
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
