use async_trait::async_trait;
use rustycog::command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{StartSyncJobRequest, SyncJobResponse},
    usecase::SyncJobUseCase,
    ApplicationError,
};

#[derive(Debug, Clone)]
pub struct StartSyncJobCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: StartSyncJobRequest,
    pub user_id: Uuid,
}

impl StartSyncJobCommand {
    #[must_use]
    pub fn new(organization_id: Uuid, request: StartSyncJobRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for StartSyncJobCommand {
    type Result = SyncJobResponse;

    fn command_type(&self) -> &'static str {
        "start_sync_job"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct StartSyncJobCommandHandler {
    sync_job_usecase: Arc<dyn SyncJobUseCase>,
}

impl StartSyncJobCommandHandler {
    pub fn new(sync_job_usecase: Arc<dyn SyncJobUseCase>) -> Self {
        Self { sync_job_usecase }
    }
}

#[async_trait]
impl CommandHandler<StartSyncJobCommand> for StartSyncJobCommandHandler {
    async fn handle(&self, command: StartSyncJobCommand) -> Result<SyncJobResponse, CommandError> {
        self.sync_job_usecase
            .start_sync_job(command.organization_id, command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("start_sync_job_failed", e.to_string()))
    }
}

pub struct SyncJobErrorMapper;

impl CommandErrorMapper for SyncJobErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        error.downcast_ref::<ApplicationError>().map_or_else(
            || CommandError::business("unknown_error", error.to_string()),
            |error| CommandError::infrastructure("sync_job_operation_failed", error.to_string()),
        )
    }
}
