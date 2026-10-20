//! Object-safe neutral port implementation; no cache or transport assumptions.

use super::SqlRuntimeControl;
use apparatus_contracts::runtime::{
    RuntimeBeginEffectRequestV1, RuntimeBindingPageV1, RuntimeBindingSnapshotV1,
    RuntimeCompleteEffectRequestV1, RuntimeControlCallerV1, RuntimeControlErrorV1,
    RuntimeControlFuture, RuntimeControlPort, RuntimeCursorV1, RuntimeEffectReservationV1,
    RuntimeInstanceRegistrationV1, RuntimeInstanceRevokeRequestV1, RuntimeInstanceSelectorV1,
    RuntimeObservationV1, RuntimeWriteAckV1, RuntimeWriterLeaseRequestV1, RuntimeWriterLeaseV1,
};
use uuid::Uuid;

impl RuntimeControlPort for SqlRuntimeControl {
    fn list_bindings<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        cursor: Option<&'a RuntimeCursorV1>,
        page_size: Option<u16>,
    ) -> RuntimeControlFuture<'a, RuntimeBindingPageV1> {
        Box::pin(async move {
            SqlRuntimeControl::list_bindings(self, caller, project_id, cursor, page_size)
                .await
                .map_err(|e| role_error(e, caller))
        })
    }
    fn get_binding<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instances: &'a RuntimeInstanceSelectorV1,
    ) -> RuntimeControlFuture<'a, RuntimeBindingSnapshotV1> {
        Box::pin(async move {
            SqlRuntimeControl::get_binding(self, caller, project_id, binding_id, instances)
                .await
                .map_err(|e| role_error(e, caller))
        })
    }
    fn writer_lease<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeWriterLeaseRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriterLeaseV1> {
        Box::pin(async move {
            SqlRuntimeControl::writer_lease(self, caller, project_id, binding_id, request)
                .await
                .map_err(|e| role_error(e, caller))
        })
    }
    fn report_observation<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeObservationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            SqlRuntimeControl::report_observation(self, caller, project_id, binding_id, request)
                .await
                .map_err(|e| role_error(e, caller))
        })
    }
    fn register_instance<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &'a RuntimeInstanceRegistrationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            SqlRuntimeControl::register_instance(
                self,
                caller,
                project_id,
                binding_id,
                instance_id,
                request,
            )
            .await
            .map_err(|e| role_error(e, caller))
        })
    }
    fn revoke_instance<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &'a RuntimeInstanceRevokeRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            SqlRuntimeControl::revoke_instance(
                self,
                caller,
                project_id,
                binding_id,
                instance_id,
                request,
            )
            .await
            .map_err(|e| role_error(e, caller))
        })
    }
    fn begin_effect<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeBeginEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeEffectReservationV1> {
        Box::pin(async move {
            SqlRuntimeControl::begin_effect(self, caller, project_id, binding_id, request)
                .await
                .map_err(|e| role_error(e, caller))
        })
    }
    fn complete_effect<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        effect_id: Uuid,
        request: &'a RuntimeCompleteEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            SqlRuntimeControl::complete_effect(
                self, caller, project_id, binding_id, effect_id, request,
            )
            .await
            .map_err(|e| role_error(e, caller))
        })
    }
}

fn role_error(
    error: RuntimeControlErrorV1,
    caller: &RuntimeControlCallerV1,
) -> RuntimeControlErrorV1 {
    error.code.error(caller.role, error.effect_outcome)
}
