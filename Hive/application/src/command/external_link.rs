use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use rustycog_command::{Command, CommandHandler, CommandError};

use crate::{
    usecase::ExternalLinkUseCase,
    dto::{
        CreateExternalLinkRequest, UpdateExternalLinkRequest, ToggleSyncRequest,
        ExternalLinkResponse, ExternalLinkListResponse, ConnectionTestResponse
    }
};

#[derive(Debug, Clone)]
pub struct CreateExternalLinkCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: CreateExternalLinkRequest,
    pub user_id: Uuid,
}

impl CreateExternalLinkCommand {
    pub fn new(organization_id: Uuid, request: CreateExternalLinkRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for CreateExternalLinkCommand {
    type Result = ExternalLinkResponse;

    fn command_type(&self) -> &'static str {
        "create_external_link"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct CreateExternalLinkCommandHandler {
    external_link_usecase: Arc<dyn ExternalLinkUseCase>,
}

impl CreateExternalLinkCommandHandler {
    pub fn new(external_link_usecase: Arc<dyn ExternalLinkUseCase>) -> Self {
        Self { external_link_usecase }
    }
}

#[async_trait]
impl CommandHandler<CreateExternalLinkCommand> for CreateExternalLinkCommandHandler {
    async fn handle(&self, command: CreateExternalLinkCommand) -> Result<ExternalLinkResponse, CommandError> {
        self.external_link_usecase
            .create_link(command.organization_id, command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("create_external_link_failed", &e.to_string()))
    }
}

pub struct ExternalLinkErrorMapper;

impl ExternalLinkErrorMapper {
    pub fn from_application_error(error: crate::ApplicationError) -> CommandError {
        CommandError::business("external_link_operation_failed", &error.to_string())
    }
} 