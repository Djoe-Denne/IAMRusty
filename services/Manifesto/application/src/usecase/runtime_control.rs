//! Control use case: a single command execution surface over the writer port.

use apparatus_contracts::runtime::{
    RuntimeBeginEffectRequestV1, RuntimeBindingPageV1, RuntimeBindingSnapshotV1,
    RuntimeCompleteEffectRequestV1, RuntimeControlCallerV1, RuntimeControlErrorV1,
    RuntimeControlPort, RuntimeCursorV1, RuntimeEffectReservationV1, RuntimeInstanceRegistrationV1,
    RuntimeInstanceRevokeRequestV1, RuntimeInstanceSelectorV1, RuntimeObservationV1,
    RuntimeWriteAckV1, RuntimeWriterLeaseRequestV1, RuntimeWriterLeaseV1,
};
use std::sync::Arc;
use uuid::Uuid;

/// Typed internal action. Deliberately not Deserialize/Debug: a body cannot
/// manufacture an origin, and capability params must not enter command logs.
pub enum RuntimeControlAction {
    List {
        project_id: Uuid,
        cursor: Option<RuntimeCursorV1>,
        page_size: Option<u16>,
    },
    Get {
        project_id: Uuid,
        binding_id: Uuid,
        instances: RuntimeInstanceSelectorV1,
    },
    Lease {
        project_id: Uuid,
        binding_id: Uuid,
        request: RuntimeWriterLeaseRequestV1,
    },
    Observation {
        project_id: Uuid,
        binding_id: Uuid,
        request: RuntimeObservationV1,
    },
    Register {
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: RuntimeInstanceRegistrationV1,
    },
    Revoke {
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: RuntimeInstanceRevokeRequestV1,
    },
    Begin {
        project_id: Uuid,
        binding_id: Uuid,
        request: RuntimeBeginEffectRequestV1,
    },
    Complete {
        project_id: Uuid,
        binding_id: Uuid,
        effect_id: Uuid,
        request: RuntimeCompleteEffectRequestV1,
    },
}

/// The wire reply remains the exact DTO for the selected route, not an envelope.
#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum RuntimeControlReply {
    Page(RuntimeBindingPageV1),
    Snapshot(RuntimeBindingSnapshotV1),
    Lease(RuntimeWriterLeaseV1),
    Ack(RuntimeWriteAckV1),
    Reservation(RuntimeEffectReservationV1),
}

/// Writer adapter supplied once by setup; never reconstructed in handlers.
pub struct RuntimeControlUseCase {
    control: Arc<dyn RuntimeControlPort>,
}
impl RuntimeControlUseCase {
    /// Construct over the writer-authoritative port.
    #[must_use]
    pub fn new(control: Arc<dyn RuntimeControlPort>) -> Self {
        Self { control }
    }

    /// Dispatch once. Errors stay structured, without SQL/free-form diagnostics.
    ///
    /// # Errors
    /// Preserves the finite runtime error taxonomy from the writer.
    pub async fn execute(
        &self,
        caller: &RuntimeControlCallerV1,
        action: RuntimeControlAction,
    ) -> Result<RuntimeControlReply, RuntimeControlErrorV1> {
        match action {
            RuntimeControlAction::List {
                project_id,
                cursor,
                page_size,
            } => self
                .control
                .list_bindings(caller, project_id, cursor.as_ref(), page_size)
                .await
                .map(RuntimeControlReply::Page),
            RuntimeControlAction::Get {
                project_id,
                binding_id,
                instances,
            } => self
                .control
                .get_binding(caller, project_id, binding_id, &instances)
                .await
                .map(RuntimeControlReply::Snapshot),
            RuntimeControlAction::Lease {
                project_id,
                binding_id,
                request,
            } => self
                .control
                .writer_lease(caller, project_id, binding_id, &request)
                .await
                .map(RuntimeControlReply::Lease),
            RuntimeControlAction::Observation {
                project_id,
                binding_id,
                request,
            } => self
                .control
                .report_observation(caller, project_id, binding_id, &request)
                .await
                .map(RuntimeControlReply::Ack),
            RuntimeControlAction::Register {
                project_id,
                binding_id,
                instance_id,
                request,
            } => self
                .control
                .register_instance(caller, project_id, binding_id, instance_id, &request)
                .await
                .map(RuntimeControlReply::Ack),
            RuntimeControlAction::Revoke {
                project_id,
                binding_id,
                instance_id,
                request,
            } => self
                .control
                .revoke_instance(caller, project_id, binding_id, instance_id, &request)
                .await
                .map(RuntimeControlReply::Ack),
            RuntimeControlAction::Begin {
                project_id,
                binding_id,
                request,
            } => self
                .control
                .begin_effect(caller, project_id, binding_id, &request)
                .await
                .map(RuntimeControlReply::Reservation),
            RuntimeControlAction::Complete {
                project_id,
                binding_id,
                effect_id,
                request,
            } => self
                .control
                .complete_effect(caller, project_id, binding_id, effect_id, &request)
                .await
                .map(RuntimeControlReply::Ack),
        }
    }
}
