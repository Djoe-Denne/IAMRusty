//! Harness in-process **réservé aux tests** (TEST-ONLY, feature `test-harness`).
//!
//! Ce module n'est compilé qu'avec `--features test-harness` ; il est absent
//! des builds de production. Il rejoue le cycle `bind` → `configure` →
//! `invoke` → `unbind` contre un KV en mémoire namespacé par `binding_id`.
//! Il qualifie les contrats wire P0, **pas** un runtime de production :
//!
//! - aucun bearer IAM, JWT, secret HMAC ou credential ;
//! - aucun accès SQL, réseau, SQS/Kafka, filesystem privilégié ;
//! - aucune isolation de production ;
//! - aucun statut d'admission ni de confiance éditoriale, jamais.
//!
//! Le KV est namespacé par l'adaptateur ([`InMemoryKv`]), jamais par un
//! préfixe choisi par l'appelant : deux bindings ne partagent aucune clé.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::error::ApparatusError;
use crate::ids::{BindingId, OperationId};
use crate::limits::{MAX_KV_KEY_LEN, MAX_KV_VALUE_BYTES};
use crate::protocol::{
    BindRequest, BindResponse, ConfigureRequest, ConfigureResponse, InvokeRequest, InvokeResponse,
    UnbindRequest, UnbindResponse,
};

/// Stockage KV en mémoire namespacé par `binding_id` (TEST-ONLY).
///
/// La clé interne est `(binding_id, key)` : l'isolation est imposée par
/// l'adaptateur, l'appelant ne choisit que `key`.
#[derive(Debug, Default)]
pub struct InMemoryKv {
    /// Entrées `(binding_id, key) -> valeur brute`.
    entries: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl InMemoryKv {
    /// Crée un stockage vide.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Lit une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est vide ou
    /// dépasse [`MAX_KV_KEY_LEN`].
    pub fn get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError> {
        validate_kv_key(key)?;
        let guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(guard
            .get(&(binding.as_str().to_owned(), key.to_owned()))
            .cloned())
    }

    /// Écrit une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est vide ou
    /// hors bornes, [`ApparatusError::PayloadTooLarge`] si la valeur dépasse
    /// [`MAX_KV_VALUE_BYTES`].
    pub fn put(&self, binding: &BindingId, key: &str, value: &[u8]) -> Result<(), ApparatusError> {
        validate_kv_key(key)?;
        if value.len() > MAX_KV_VALUE_BYTES {
            return Err(ApparatusError::PayloadTooLarge {
                max: MAX_KV_VALUE_BYTES,
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

    /// Supprime une valeur pour un binding.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si la clé est vide ou
    /// dépasse [`MAX_KV_KEY_LEN`].
    pub fn delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError> {
        validate_kv_key(key)?;
        let mut guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(guard
            .remove(&(binding.as_str().to_owned(), key.to_owned()))
            .is_some())
    }

    /// Liste les clés d'un binding (triées, sans fuite inter-bindings).
    pub fn list_keys(&self, binding: &BindingId) -> Vec<String> {
        let mut keys: Vec<String> = {
            let guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
            guard
                .keys()
                .filter_map(|(owner, key)| {
                    if owner == binding.as_str() {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };
        keys.sort();
        keys
    }

    /// Purge toutes les entrées d'un binding (appelée par `unbind`).
    pub fn purge_binding(&self, binding: &BindingId) {
        let mut guard = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        guard.retain(|(owner, _), _| owner != binding.as_str());
    }
}

/// Valide une clé KV : non vide, bornée.
fn validate_kv_key(key: &str) -> Result<(), ApparatusError> {
    if key.is_empty() || key.chars().count() > MAX_KV_KEY_LEN {
        return Err(ApparatusError::InvalidOperation {
            reason: "kv key length out of bounds".to_owned(),
        });
    }
    Ok(())
}

/// Harness de test rejouant le cycle de vie Apparatus (TEST-ONLY).
///
/// Idempotence : chaque `operation_id` déjà vu rejoue la réponse stockée
/// sans nouvel effet de bord.
#[derive(Debug, Default)]
pub struct TestHarness {
    /// KV partagé, namespacé par binding.
    kv: InMemoryKv,
    /// Bindings connus (`binding_id` présent ⇒ attaché).
    bindings: Mutex<HashMap<String, bool>>,
    /// Réponses rejouables par `operation_id`.
    replay_bind: Mutex<HashMap<String, BindResponse>>,
    /// Réponses rejouables `configure` par `operation_id`.
    replay_configure: Mutex<HashMap<String, ConfigureResponse>>,
    /// Réponses rejouables `invoke` par `operation_id`.
    replay_invoke: Mutex<HashMap<String, InvokeResponse>>,
    /// Réponses rejouables `unbind` par `operation_id`.
    replay_unbind: Mutex<HashMap<String, UnbindResponse>>,
}

impl TestHarness {
    /// Crée un harness vierge.
    #[must_use]
    pub fn new() -> Self {
        Self {
            kv: InMemoryKv::new(),
            bindings: Mutex::new(HashMap::new()),
            replay_bind: Mutex::new(HashMap::new()),
            replay_configure: Mutex::new(HashMap::new()),
            replay_invoke: Mutex::new(HashMap::new()),
            replay_unbind: Mutex::new(HashMap::new()),
        }
    }

    /// Accède au KV sous-jacent (assertions de tests).
    #[must_use]
    pub const fn kv(&self) -> &InMemoryKv {
        &self.kv
    }

    /// Indique si un binding est attaché.
    #[must_use]
    pub fn is_bound(&self, binding: &BindingId) -> bool {
        let guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
        guard.get(binding.as_str()).copied().unwrap_or(false)
    }

    /// Exécute `bind` (idempotent par `operation_id`).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::ConfigTooLarge`] si la configuration
    /// initiale dépasse la borne.
    pub fn bind(&self, request: &BindRequest) -> Result<BindResponse, ApparatusError> {
        request.validate()?;
        let replay_key = operation_key(&request.operation_id);
        {
            let guard = self
                .replay_bind
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(stored) = guard.get(&replay_key) {
                return Ok(stored.clone());
            }
        }
        {
            let mut guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
            guard.insert(request.binding_id.as_str().to_owned(), true);
        }
        let response = BindResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            applied: true,
        };
        {
            let mut guard = self
                .replay_bind
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            guard.insert(replay_key, response.clone());
        }
        Ok(response)
    }

    /// Exécute `configure` (idempotent par `operation_id`).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si le binding est inconnu,
    /// [`ApparatusError::ConfigTooLarge`] si la configuration dépasse la borne.
    pub fn configure(
        &self,
        request: &ConfigureRequest,
    ) -> Result<ConfigureResponse, ApparatusError> {
        request.validate()?;
        let replay_key = operation_key(&request.operation_id);
        {
            let guard = self
                .replay_configure
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(stored) = guard.get(&replay_key) {
                return Ok(stored.clone());
            }
        }
        self.require_bound(&request.binding_id)?;
        let response = ConfigureResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            applied: true,
        };
        {
            let mut guard = self
                .replay_configure
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            guard.insert(replay_key, response.clone());
        }
        Ok(response)
    }

    /// Exécute `invoke` (idempotent par `operation_id`).
    ///
    /// Opérations supportées : `kv.get`, `kv.put`, `kv.delete`.
    /// Paramètres : `{"key": "…", "value": "…"}` (`value` requis pour `kv.put`).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si le binding est inconnu,
    /// pour une opération ou des paramètres invalides,
    /// [`ApparatusError::PayloadTooLarge`] hors bornes.
    pub fn invoke(&self, request: &InvokeRequest) -> Result<InvokeResponse, ApparatusError> {
        request.validate()?;
        let replay_key = operation_key(&request.operation_id);
        {
            let guard = self
                .replay_invoke
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(stored) = guard.get(&replay_key) {
                return Ok(stored.clone());
            }
        }
        self.require_bound(&request.binding_id)?;
        let result = self.dispatch_kv(request)?;
        let response = InvokeResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            result,
        };
        response.validate()?;
        {
            let mut guard = self
                .replay_invoke
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            guard.insert(replay_key, response.clone());
        }
        Ok(response)
    }

    /// Exécute `unbind` (idempotent par `operation_id`, purge le namespace KV).
    ///
    /// Le rejeu du même `operation_id` rejoue la réponse stockée même si le
    /// binding a déjà été purgé ; un nouvel `operation_id` sur un binding
    /// inconnu est rejeté.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidOperation`] si le binding est inconnu
    /// (hors rejeu du même `operation_id`).
    pub fn unbind(&self, request: &UnbindRequest) -> Result<UnbindResponse, ApparatusError> {
        request.validate()?;
        let replay_key = operation_key(&request.operation_id);
        {
            let guard = self
                .replay_unbind
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(stored) = guard.get(&replay_key) {
                return Ok(stored.clone());
            }
        }
        self.require_bound(&request.binding_id)?;
        {
            let mut guard = self.bindings.lock().unwrap_or_else(PoisonError::into_inner);
            guard.remove(request.binding_id.as_str());
        }
        self.kv.purge_binding(&request.binding_id);
        let response = UnbindResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            applied: true,
        };
        {
            let mut guard = self
                .replay_unbind
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            guard.insert(replay_key, response.clone());
        }
        Ok(response)
    }

    /// Exige un binding attaché.
    fn require_bound(&self, binding: &BindingId) -> Result<(), ApparatusError> {
        if self.is_bound(binding) {
            Ok(())
        } else {
            Err(ApparatusError::InvalidOperation {
                reason: "unknown binding".to_owned(),
            })
        }
    }

    /// Dispatche les opérations KV (`kv.get` / `kv.put` / `kv.delete`).
    fn dispatch_kv(&self, request: &InvokeRequest) -> Result<serde_json::Value, ApparatusError> {
        match request.operation.as_str() {
            "kv.get" => {
                let key = invoke_key(&request.params)?;
                let found = self.kv.get(&request.binding_id, key)?;
                Ok(found.map_or_else(
                    || serde_json::json!({"found": false}),
                    |bytes| {
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        serde_json::json!({"found": true, "value": text})
                    },
                ))
            }
            "kv.put" => {
                let key = invoke_key(&request.params)?;
                let value = invoke_value(&request.params)?;
                self.kv.put(&request.binding_id, key, value.as_bytes())?;
                Ok(serde_json::json!({"stored": true}))
            }
            "kv.delete" => {
                let key = invoke_key(&request.params)?;
                let removed = self.kv.delete(&request.binding_id, key)?;
                Ok(serde_json::json!({"removed": removed}))
            }
            _ => Err(ApparatusError::InvalidOperation {
                reason: "unknown harness operation".to_owned(),
            }),
        }
    }
}

/// Clé de rejeu stable pour un `operation_id`.
fn operation_key(operation: &OperationId) -> String {
    operation.to_string()
}

/// Extrait `key` des paramètres d'invocation.
fn invoke_key(params: &serde_json::Value) -> Result<&str, ApparatusError> {
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

/// Extrait `value` des paramètres d'invocation.
fn invoke_value(params: &serde_json::Value) -> Result<&str, ApparatusError> {
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
