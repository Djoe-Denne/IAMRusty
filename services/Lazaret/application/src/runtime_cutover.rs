//! Per-process deny-before-migrate bootstrap (§13.3-B). No HTTP/queue entry point.
//! Maintenance approval is a mandatory trusted composition-root collaborator,
//! never a bool, JSON receipt, caller header, or persistent seal used on its own.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use apparatus_contracts::runtime::*;
use async_trait::async_trait;
use lazaret_domain::runtime::{
    RuntimeAdmissionGuard, RuntimeEnrollment, RuntimeError, RuntimeStore,
};
use uuid::Uuid;

/// Authenticated local maintenance seam. A real implementation must check the
/// operator's authority, exact deployment/database identity, exhaustive writer
/// inventory, suspended migrations/admissions and retirement of old D binaries.
/// Re-check these conditions before opening readiness. No default allow exists.
#[async_trait]
pub trait RuntimeBootstrapMaintenance: Send + Sync {
    async fn verify_inventory(
        &self,
        projects: &[Uuid],
        inventory: &[InstallRuntimeLegacyFenceV1],
    ) -> Result<(), RuntimeError>;
    async fn verify_before_open(&self) -> Result<(), RuntimeError>;
}

/// Safe default for deployments without the authenticated maintenance runner.
pub struct RejectRuntimeBootstrapMaintenance;

#[async_trait]
impl RuntimeBootstrapMaintenance for RejectRuntimeBootstrapMaintenance {
    async fn verify_inventory(
        &self,
        _: &[Uuid],
        _: &[InstallRuntimeLegacyFenceV1],
    ) -> Result<(), RuntimeError> {
        Err(RuntimeError::RuntimeServiceForbidden)
    }
    async fn verify_before_open(&self) -> Result<(), RuntimeError> {
        Err(RuntimeError::RuntimeServiceForbidden)
    }
}

pub struct RuntimeCutoverGuard {
    store: Arc<dyn RuntimeStore>,
    control: Arc<dyn RuntimeControlPort>,
    // Per-process readiness only, NEVER distributed writer fencing.
    readiness: Mutex<BootstrapReadiness>,
}

struct BootstrapReadiness {
    generation: u64,
    ready: bool,
    exhausted: bool,
}

impl RuntimeCutoverGuard {
    pub fn new(store: Arc<dyn RuntimeStore>, control: Arc<dyn RuntimeControlPort>) -> Self {
        Self {
            store,
            control,
            readiness: Mutex::new(BootstrapReadiness {
                generation: 0,
                ready: false,
                exhausted: false,
            }),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.readiness
            .lock()
            .is_ok_and(|state| state.ready && !state.exhausted)
    }

    pub fn close(&self) {
        let _ = self.close_generation();
    }

    fn close_generation(&self) -> Result<u64, RuntimeError> {
        let mut state = self
            .readiness
            .lock()
            .map_err(|_| RuntimeError::RuntimeInternal)?;
        state.ready = false;
        let Some(next) = state.generation.checked_add(1) else {
            state.exhausted = true;
            return Err(RuntimeError::RuntimeInternal);
        };
        state.generation = next;
        Ok(next)
    }

    /// Root/maintenance-only operation. Every process repeats this after startup;
    /// installed fences are never removed when a scan fails or returns nothing.
    pub async fn reconcile(
        &self,
        maintenance: &dyn RuntimeBootstrapMaintenance,
        projects: &[Uuid],
        inventory: &[InstallRuntimeLegacyFenceV1],
    ) -> Result<(), RuntimeError> {
        let generation = self.close_generation()?;
        maintenance.verify_inventory(projects, inventory).await?;
        let scope: BTreeSet<_> = projects.iter().copied().collect();
        if scope.is_empty() || scope.len() != projects.len() {
            return Err(RuntimeError::RuntimeInvalidRequest);
        }
        let mut expected = BTreeMap::new();
        for command in inventory {
            if command.schema_version != 1
                || !scope.contains(&command.project_id)
                || expected
                    .insert(command.binding_id, command.project_id)
                    .is_some()
            {
                return Err(RuntimeError::RuntimeInvalidRequest);
            }
        }
        let caller = RuntimeControlCallerV1 {
            role: RuntimeServiceRoleV1::Lazaret,
        };
        let mut observed = BTreeSet::new();
        for project in projects {
            let mut cursor = None;
            let mut cursors = BTreeSet::new();
            loop {
                let page = self
                    .control
                    .list_bindings(&caller, *project, cursor.as_ref(), Some(100))
                    .await
                    .map_err(|error| error.code)?;
                if page.schema_version != 1 || page.items.len() > 100 {
                    return Err(RuntimeError::RuntimeAuthorityProtocolError);
                }
                for binding in &page.items {
                    if binding.project_id != *project
                        || binding.instances.is_some()
                        || expected.get(&binding.binding_id) != Some(project)
                        || !observed.insert(binding.binding_id)
                    {
                        return Err(RuntimeError::RuntimeAuthorityProtocolError);
                    }
                }
                match page.next_cursor {
                    None => break,
                    Some(next) if !page.items.is_empty() && cursors.insert(next.0.clone()) => {
                        cursor = Some(next);
                    }
                    Some(_) => return Err(RuntimeError::RuntimeAuthorityProtocolError),
                }
            }
        }
        // The trusted inventory may also include bindings about to be prepared;
        // deny-before-migrate deliberately permits D's fence to precede C's row.
        for command in inventory {
            let fence = self.store.install_legacy_fence(command).await?;
            if fence.project_id != command.project_id
                || fence.binding_id != command.binding_id
                || fence.migration_id != command.migration_id
                || fence.inventory_sha256 != command.inventory_sha256
            {
                return Err(RuntimeError::RuntimeAuthorityProtocolError);
            }
        }
        // Exhaustive local reconciliation too: an unexplained old fence is not
        // silently dropped or treated as an inventory match after restoration.
        for project in projects {
            let mut after = None;
            loop {
                let page = self.store.list_cutover_fences(*project, after, 100).await?;
                if page.is_empty() {
                    break;
                }
                for fence in &page {
                    let command = inventory
                        .iter()
                        .find(|item| item.binding_id == fence.binding_id)
                        .ok_or(RuntimeError::RuntimeProjectionUnavailable)?;
                    if fence.project_id != *project
                        || fence.migration_id != command.migration_id
                        || fence.inventory_sha256 != command.inventory_sha256
                        || after.is_some_and(|previous| fence.binding_id <= previous)
                    {
                        return Err(RuntimeError::RuntimeAuthorityProtocolError);
                    }
                    after = Some(fence.binding_id);
                }
                if page.len() < 100 {
                    break;
                }
            }
        }
        maintenance.verify_before_open().await?;
        let mut state = self
            .readiness
            .lock()
            .map_err(|_| RuntimeError::RuntimeInternal)?;
        if state.generation != generation || state.exhausted {
            return Err(RuntimeError::RuntimeProjectionUnavailable);
        }
        state.ready = true;
        Ok(())
    }

    fn require_ready(&self) -> Result<(), RuntimeError> {
        if self.is_ready() {
            Ok(())
        } else {
            Err(RuntimeError::RuntimeProjectionUnavailable)
        }
    }
}

#[async_trait]
impl RuntimeAdmissionGuard for RuntimeCutoverGuard {
    async fn require_runtime(&self, enrollment: &RuntimeEnrollment) -> Result<(), RuntimeError> {
        if enrollment.closed {
            return Err(RuntimeError::RuntimeIdentityRevoked);
        }
        self.require_binding(enrollment.project_id, enrollment.identity.binding)
            .await
    }

    async fn require_binding(
        &self,
        project_id: Uuid,
        binding_id: Uuid,
    ) -> Result<(), RuntimeError> {
        self.require_ready()?;
        let fence = self
            .store
            .cutover_fence(binding_id)
            .await?
            .ok_or(RuntimeError::RuntimeProjectionUnavailable)?;
        if fence.project_id != project_id || fence.binding_id != binding_id {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        self.require_ready()
    }

    async fn require_legacy(&self, binding_id: Uuid) -> Result<(), RuntimeError> {
        self.require_ready()?;
        self.store.admit_legacy(binding_id).await?;
        self.require_ready()
    }
}
