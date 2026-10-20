//! Non-Clone runtime command; all handlers execute_once, never business retries.

use crate::usecase::{RuntimeControlAction, RuntimeControlReply, RuntimeControlUseCase};
use apparatus_contracts::runtime::{RuntimeControlCallerV1, RuntimeControlErrorV1};
use async_trait::async_trait;
use rustycog::command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

/// Origin comes from mTLS middleware, never deserialized from an action/body.
pub struct RuntimeControlCommand {
    command_id: Uuid,
    caller: RuntimeControlCallerV1,
    action: RuntimeControlAction,
}
impl RuntimeControlCommand {
    /// Build after transport authentication and route-specific role validation.
    #[must_use]
    pub fn new(caller: RuntimeControlCallerV1, action: RuntimeControlAction) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            caller,
            action,
        }
    }
}
impl std::fmt::Debug for RuntimeControlCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeControlCommand")
            .field("command_id", &self.command_id)
            .finish_non_exhaustive()
    }
}
#[async_trait]
impl Command for RuntimeControlCommand {
    type Result = Result<RuntimeControlReply, RuntimeControlErrorV1>;
    fn command_type(&self) -> &'static str {
        "apparatus_runtime_control_v1"
    }
    fn command_id(&self) -> Uuid {
        self.command_id
    }
    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

/// One shared use case, not a per-request SQL client.
pub struct RuntimeControlCommandHandler {
    usecase: Arc<RuntimeControlUseCase>,
}
impl RuntimeControlCommandHandler {
    /// Construct at the composition root.
    #[must_use]
    pub fn new(usecase: Arc<RuntimeControlUseCase>) -> Self {
        Self { usecase }
    }
}
#[async_trait]
impl CommandHandler<RuntimeControlCommand> for RuntimeControlCommandHandler {
    async fn handle(
        &self,
        command: RuntimeControlCommand,
    ) -> Result<<RuntimeControlCommand as Command>::Result, CommandError> {
        // Business refusals are data, not a framework retry signal.
        Ok(self.usecase.execute(&command.caller, command.action).await)
    }
}

/// Outer infrastructure errors remain opaque; runtime business errors never
/// pass through this mapper and keep their exact versioned DTO.
pub struct RuntimeControlCommandErrorMapper;
impl CommandErrorMapper for RuntimeControlCommandErrorMapper {
    fn map_error(&self, _error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        CommandError::business("runtime_command_error", "runtime_command_error")
    }
}
