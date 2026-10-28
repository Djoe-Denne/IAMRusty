//! Local maintenance commands, intentionally non-Clone/non-Serde. They are
//! registered only in setup's private maintenance runner, never HTTP/queue.
use apparatus_contracts::runtime::*;
use async_trait::async_trait;
use rustycog::command::{Command, CommandError, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

pub enum RuntimeMaintenanceAction {
    Inspect(RuntimeBindingCutoverPlanV1),
    Fence(FenceRuntimeBindingV1),
    Challenge {
        plan: RuntimeBindingCutoverPlanV1,
        action: RuntimeAdmissionActionV1,
        expected: RuntimeCasV1,
        service_uid: Option<String>,
        revision_uid: Option<String>,
    },
    Prepare {
        plan: RuntimeBindingCutoverPlanV1,
        command: PrepareRuntimeBindingV1,
        challenge_id: Uuid,
        proof: String,
    },
    ResumePrepare {
        plan: RuntimeBindingCutoverPlanV1,
        command: PrepareRuntimeBindingV1,
        challenge_id: Uuid,
        proof: String,
    },
    Open {
        plan: RuntimeBindingCutoverPlanV1,
        command: OpenRuntimeBindingV1,
        challenge_id: Uuid,
        proof: String,
    },
    BlockRelease {
        plan: RuntimeBindingCutoverPlanV1,
        record_id: Uuid,
        revision: RuntimeCounterV1,
    },
}

/// Committed barrier DATA, not an authenticated D/Adm-A maintenance capability.
pub struct RuntimeAdmissionClosureAck {
    pub authority_id: Uuid,
    pub release: RuntimeDigestV1,
    pub record_id: Uuid,
    pub maximum_revision: RuntimeCounterV1,
    pub closed_bindings: Vec<(Uuid, Uuid)>,
}

pub enum RuntimeMaintenanceReply {
    Inspection(RuntimeMaintenanceInspection),
    Cas(RuntimeCasV1),
    Challenge(RuntimeAdmissionChallengeV1),
    Permit(RuntimeBindingPreparationPermitV1),
    Closure(RuntimeAdmissionClosureAck),
}

/// Local recovery data. It never authenticates an ack when copied into a file.
pub struct RuntimeMaintenanceInspection {
    pub cas: RuntimeCasV1,
    pub phase: Option<String>,
    pub journal_json: String,
    pub consumed_challenges: Vec<Uuid>,
}

pub trait RuntimeMaintenanceExecutor: Send + Sync {
    fn execute_maintenance<'a>(
        &'a self,
        action: RuntimeMaintenanceAction,
    ) -> RuntimeControlFuture<'a, RuntimeMaintenanceReply>;
}

pub struct RuntimeMaintenanceCommand {
    id: Uuid,
    action: RuntimeMaintenanceAction,
}
impl RuntimeMaintenanceCommand {
    pub fn new(action: RuntimeMaintenanceAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            action,
        }
    }
}
impl std::fmt::Debug for RuntimeMaintenanceCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeMaintenanceCommand")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
#[async_trait]
impl Command for RuntimeMaintenanceCommand {
    type Result = Result<RuntimeMaintenanceReply, RuntimeControlErrorV1>;
    fn command_type(&self) -> &'static str {
        "apparatus_runtime_maintenance_v1"
    }
    fn command_id(&self) -> Uuid {
        self.id
    }
    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct RuntimeMaintenanceCommandHandler {
    executor: Arc<dyn RuntimeMaintenanceExecutor>,
}
impl RuntimeMaintenanceCommandHandler {
    pub fn new(executor: Arc<dyn RuntimeMaintenanceExecutor>) -> Self {
        Self { executor }
    }
}
#[async_trait]
impl CommandHandler<RuntimeMaintenanceCommand> for RuntimeMaintenanceCommandHandler {
    async fn handle(
        &self,
        command: RuntimeMaintenanceCommand,
    ) -> Result<<RuntimeMaintenanceCommand as Command>::Result, CommandError> {
        Ok(self.executor.execute_maintenance(command.action).await)
    }
}
