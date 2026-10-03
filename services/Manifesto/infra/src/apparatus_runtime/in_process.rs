//! Double in-process déterministe du port [`ApparatusRuntime`].
//!
//! `Mutex` + `HashMap` : une instance par `binding_id`. Le rejeu du même
//! `operation_id` renvoie `applied = false` sans second effet.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use apparatus_contracts::{
    ApparatusError, ApparatusRuntime, BindRequest, BindResponse, BindingId, ConfigureRequest,
    ConfigureResponse, RuntimeObservation, UnbindRequest, UnbindResponse,
};

#[derive(Debug)]
struct LiveInstance {
    digest: String,
    generation: u64,
}

#[derive(Debug)]
struct Inner {
    instances: HashMap<String, LiveInstance>,
    seen_bind: HashMap<String, BindResponse>,
    seen_configure: HashMap<String, ConfigureResponse>,
    seen_unbind: HashMap<String, UnbindResponse>,
    applied_bind: u64,
    teardown_calls: u64,
}

/// Runtime Apparatus in-process (double de test / P2).
#[derive(Debug)]
pub struct InProcessApparatusRuntime {
    inner: Mutex<Inner>,
}

impl Default for InProcessApparatusRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessApparatusRuntime {
    /// Crée un runtime vide.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                instances: HashMap::new(),
                seen_bind: HashMap::new(),
                seen_configure: HashMap::new(),
                seen_unbind: HashMap::new(),
                applied_bind: 0,
                teardown_calls: 0,
            }),
        }
    }

    /// Nombre d'instances actuellement attachées.
    #[must_use]
    pub fn instance_count(&self) -> usize {
        self.lock().instances.len()
    }

    /// Nombre de `bind` qui ont réellement appliqué un effet.
    #[must_use]
    pub fn applied_bind_count(&self) -> u64 {
        self.lock().applied_bind
    }

    /// Nombre d'appels `teardown` (y compris no-op).
    #[must_use]
    pub fn teardown_call_count(&self) -> u64 {
        self.lock().teardown_calls
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ApparatusRuntime for InProcessApparatusRuntime {
    fn bind(&self, req: &BindRequest) -> Result<BindResponse, ApparatusError> {
        req.validate()?;
        let op_key = req.operation_id.to_string();
        let mut inner = self.lock();
        if let Some(stored) = inner.seen_bind.get(&op_key) {
            let mut replay = stored.clone();
            replay.applied = false;
            return Ok(replay);
        }
        inner.instances.insert(
            req.binding_id.as_str().to_owned(),
            LiveInstance {
                digest: req.release_digest.as_str().to_owned(),
                generation: 1,
            },
        );
        inner.applied_bind = inner.applied_bind.saturating_add(1);
        let response = BindResponse {
            binding_id: req.binding_id.clone(),
            operation_id: req.operation_id,
            applied: true,
        };
        inner.seen_bind.insert(op_key, response.clone());
        Ok(response)
    }

    fn configure(&self, req: &ConfigureRequest) -> Result<ConfigureResponse, ApparatusError> {
        req.validate()?;
        let op_key = req.operation_id.to_string();
        let mut inner = self.lock();
        if let Some(stored) = inner.seen_configure.get(&op_key) {
            let mut replay = stored.clone();
            replay.applied = false;
            return Ok(replay);
        }
        if !inner.instances.contains_key(req.binding_id.as_str()) {
            return Err(ApparatusError::InvalidOperation {
                reason: "unknown binding".to_owned(),
            });
        }
        let response = ConfigureResponse {
            binding_id: req.binding_id.clone(),
            operation_id: req.operation_id,
            applied: true,
        };
        inner.seen_configure.insert(op_key, response.clone());
        Ok(response)
    }

    fn unbind(&self, req: &UnbindRequest) -> Result<UnbindResponse, ApparatusError> {
        req.validate()?;
        let op_key = req.operation_id.to_string();
        let mut inner = self.lock();
        if let Some(stored) = inner.seen_unbind.get(&op_key) {
            let mut replay = stored.clone();
            replay.applied = false;
            return Ok(replay);
        }
        if inner.instances.remove(req.binding_id.as_str()).is_none() {
            return Err(ApparatusError::InvalidOperation {
                reason: "unknown binding".to_owned(),
            });
        }
        let response = UnbindResponse {
            binding_id: req.binding_id.clone(),
            operation_id: req.operation_id,
            applied: true,
        };
        inner.seen_unbind.insert(op_key, response.clone());
        Ok(response)
    }

    fn observe(&self, binding: &BindingId) -> Result<RuntimeObservation, ApparatusError> {
        let inner = self.lock();
        Ok(inner.instances.get(binding.as_str()).map_or(
            RuntimeObservation {
                digest: None,
                generation: 0,
            },
            |live| RuntimeObservation {
                digest: Some(live.digest.clone()),
                generation: live.generation,
            },
        ))
    }

    fn teardown(&self, binding: &BindingId) -> Result<(), ApparatusError> {
        let mut inner = self.lock();
        inner.teardown_calls = inner.teardown_calls.saturating_add(1);
        inner.instances.remove(binding.as_str());
        Ok(())
    }
}
