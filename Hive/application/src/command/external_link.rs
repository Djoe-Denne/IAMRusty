use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        ConnectionTestResponse, CreateExternalLinkRequest, ExternalLinkListResponse,
        ExternalLinkResponse, ToggleSyncRequest, UpdateExternalLinkRequest,
    },
    usecase::ExternalLinkUseCase,
    ApplicationError,
};

#[derive(Debug, Clone)]
pub struct CreateExternalLinkCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: CreateExternalLinkRequest,
    pub user_id: Uuid,
}

impl CreateExternalLinkCommand {
    pub fn new(organization_id: Uuid, request: &CreateExternalLinkRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request: request.clone(),
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
        Self {
            external_link_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<CreateExternalLinkCommand> for CreateExternalLinkCommandHandler {
    async fn handle(
        &self,
        command: CreateExternalLinkCommand,
    ) -> Result<ExternalLinkResponse, CommandError> {
        self.external_link_usecase
            .create_link(command.organization_id, &command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("create_external_link_failed", &e.to_string()))
    }
}

pub struct ExternalLinkErrorMapper;

impl CommandErrorMapper for ExternalLinkErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(error) = error.downcast_ref::<ApplicationError>() {
            match error {
                ApplicationError::Domain(domain_error) => {
                    CommandError::business("domain_error", &domain_error.to_string())
                }
                ApplicationError::ValidationError(_) => {
                    CommandError::validation("validation_failed", &error.to_string())
                }
                ApplicationError::ExternalService { .. } => {
                    CommandError::infrastructure("external_error", &error.to_string())
                }
                ApplicationError::ConcurrentOperation { .. } => {
                    CommandError::business("concurrent_operation", &error.to_string())
                }
                ApplicationError::RateLimit { .. } => {
                    CommandError::business("rate_limit", &error.to_string())
                }
                ApplicationError::Internal { .. } => {
                    CommandError::infrastructure("internal_error", &error.to_string())
                }
            }
        } else {
            CommandError::infrastructure("unknown_error", &error.to_string())
        }
    }
}
