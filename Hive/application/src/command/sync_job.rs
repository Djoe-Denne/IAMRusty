use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        StartSyncJobRequest, SyncJobListResponse, SyncJobLogsResponse, SyncJobResponse,
        SyncJobStatusResponse,
    },
    usecase::SyncJobUseCase,
};

#[derive(Debug, Clone)]
pub struct StartSyncJobCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: StartSyncJobRequest,
}

impl StartSyncJobCommand {
    pub fn new(organization_id: Uuid, request: StartSyncJobRequest) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
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
            .start_sync_job(command.organization_id, command.request)
            .await
            .map_err(|e| CommandError::business("start_sync_job_failed", &e.to_string()))
    }
}

pub struct SyncJobErrorMapper;

impl SyncJobErrorMapper {
    pub fn from_application_error(error: crate::ApplicationError) -> CommandError {
        CommandError::business("sync_job_operation_failed", &error.to_string())
    }
}
