use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{AddComponentRequest, ComponentListResponse, ComponentResponse, UpdateComponentRequest},
    usecase::ComponentUseCase,
    ApplicationError,
};

// =============================================================================
// Error Mapper
// =============================================================================

pub struct ComponentErrorMapper;

impl CommandErrorMapper for ComponentErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(error) = error.downcast_ref::<ApplicationError>() {
            match error {
                ApplicationError::Domain(domain_error) => {
                    CommandError::business("domain_error", domain_error.to_string())
                }
                ApplicationError::Validation(msg) => {
                    CommandError::validation("validation_failed", msg)
                }
                ApplicationError::NotFound(msg) => CommandError::business("not_found", msg),
                ApplicationError::AlreadyExists(msg) => {
                    CommandError::business("already_exists", msg)
                }
                ApplicationError::Internal(msg) => {
                    CommandError::infrastructure("internal_error", msg)
                }
            }
        } else {
            CommandError::business("unknown_error", error.to_string())
        }
    }
}

// =============================================================================
// Add Component Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct AddComponentCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub request: AddComponentRequest,
    pub user_id: Uuid,
}

impl AddComponentCommand {
    #[must_use]
    pub fn new(project_id: Uuid, request: AddComponentRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for AddComponentCommand {
    type Result = ComponentResponse;

    fn command_type(&self) -> &'static str {
        "add_component"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.component_type.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_component_type",
                "Component type cannot be empty",
            ));
        }
        Ok(())
    }
}

pub struct AddComponentCommandHandler {
    component_usecase: Arc<dyn ComponentUseCase>,
}

impl AddComponentCommandHandler {
    pub fn new(component_usecase: Arc<dyn ComponentUseCase>) -> Self {
        Self { component_usecase }
    }
}

#[async_trait]
impl CommandHandler<AddComponentCommand> for AddComponentCommandHandler {
    async fn handle(
        &self,
        command: AddComponentCommand,
    ) -> Result<ComponentResponse, CommandError> {
        self.component_usecase
            .add_component(command.project_id, &command.request, command.user_id)
            .await
            .map_err(CommandError::from)
    }
}

// =============================================================================
// Get Component Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct GetComponentCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub component_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl GetComponentCommand {
    #[must_use]
    pub fn new(project_id: Uuid, component_id: Uuid, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            component_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for GetComponentCommand {
    type Result = ComponentResponse;

    fn command_type(&self) -> &'static str {
        "get_component"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct GetComponentCommandHandler {
    component_usecase: Arc<dyn ComponentUseCase>,
}

impl GetComponentCommandHandler {
    pub fn new(component_usecase: Arc<dyn ComponentUseCase>) -> Self {
        Self { component_usecase }
    }
}

#[async_trait]
impl CommandHandler<GetComponentCommand> for GetComponentCommandHandler {
    async fn handle(
        &self,
        command: GetComponentCommand,
    ) -> Result<ComponentResponse, CommandError> {
        self.component_usecase
            .get_component(command.project_id, command.component_id, command.user_id)
            .await
            .map_err(CommandError::from)
    }
}

// =============================================================================
// List Components Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct ListComponentsCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl ListComponentsCommand {
    #[must_use]
    pub fn new(project_id: Uuid, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for ListComponentsCommand {
    type Result = ComponentListResponse;

    fn command_type(&self) -> &'static str {
        "list_components"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ListComponentsCommandHandler {
    component_usecase: Arc<dyn ComponentUseCase>,
}

impl ListComponentsCommandHandler {
    pub fn new(component_usecase: Arc<dyn ComponentUseCase>) -> Self {
        Self { component_usecase }
    }
}

#[async_trait]
impl CommandHandler<ListComponentsCommand> for ListComponentsCommandHandler {
    async fn handle(
        &self,
        command: ListComponentsCommand,
    ) -> Result<ComponentListResponse, CommandError> {
        self.component_usecase
            .list_components(command.project_id, command.user_id)
            .await
            .map_err(CommandError::from)
    }
}

// =============================================================================
// Update Component Status Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct UpdateComponentStatusCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub component_id: Uuid,
    pub request: UpdateComponentRequest,
    pub user_id: Uuid,
}

impl UpdateComponentStatusCommand {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        component_id: Uuid,
        request: UpdateComponentRequest,
        user_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            component_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for UpdateComponentStatusCommand {
    type Result = ComponentResponse;

    fn command_type(&self) -> &'static str {
        "update_component_status"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.status.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_status",
                "Status cannot be empty",
            ));
        }
        Ok(())
    }
}

pub struct UpdateComponentStatusCommandHandler {
    component_usecase: Arc<dyn ComponentUseCase>,
}

impl UpdateComponentStatusCommandHandler {
    pub fn new(component_usecase: Arc<dyn ComponentUseCase>) -> Self {
        Self { component_usecase }
    }
}

#[async_trait]
impl CommandHandler<UpdateComponentStatusCommand> for UpdateComponentStatusCommandHandler {
    async fn handle(
        &self,
        command: UpdateComponentStatusCommand,
    ) -> Result<ComponentResponse, CommandError> {
        self.component_usecase
            .update_component_status(
                command.project_id,
                command.component_id,
                &command.request,
                command.user_id,
            )
            .await
            .map_err(CommandError::from)
    }
}

// =============================================================================
// Remove Component Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct RemoveComponentCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub component_id: Uuid,
    pub user_id: Uuid,
}

impl RemoveComponentCommand {
    #[must_use]
    pub fn new(project_id: Uuid, component_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            component_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for RemoveComponentCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "remove_component"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct RemoveComponentCommandHandler {
    component_usecase: Arc<dyn ComponentUseCase>,
}

impl RemoveComponentCommandHandler {
    pub fn new(component_usecase: Arc<dyn ComponentUseCase>) -> Self {
        Self { component_usecase }
    }
}

#[async_trait]
impl CommandHandler<RemoveComponentCommand> for RemoveComponentCommandHandler {
    async fn handle(&self, command: RemoveComponentCommand) -> Result<(), CommandError> {
        self.component_usecase
            .remove_component(command.project_id, command.component_id, command.user_id)
            .await
            .map_err(CommandError::from)
    }
}
