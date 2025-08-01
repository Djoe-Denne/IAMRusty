use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        CreateOrganizationRequest, OrganizationListResponse, OrganizationResponse,
        OrganizationSearchRequest, PaginationRequest, UpdateOrganizationRequest,
    },
    usecase::OrganizationUseCase,
    ApplicationError,
};

// =============================================================================
// Create Organization Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct CreateOrganizationCommand {
    pub command_id: Uuid,
    pub request: CreateOrganizationRequest,
    pub user_id: Uuid,
}

impl CreateOrganizationCommand {
    pub fn new(request: CreateOrganizationRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for CreateOrganizationCommand {
    type Result = OrganizationResponse;

    fn command_type(&self) -> &'static str {
        "create_organization"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.name.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_name",
                "Organization name cannot be empty",
            ));
        }

        if self.request.slug.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_slug",
                "Organization slug cannot be empty",
            ));
        }

        Ok(())
    }
}

pub struct CreateOrganizationCommandHandler {
    organization_usecase: Arc<dyn OrganizationUseCase>,
}

impl CreateOrganizationCommandHandler {
    pub fn new(organization_usecase: Arc<dyn OrganizationUseCase>) -> Self {
        Self {
            organization_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<CreateOrganizationCommand> for CreateOrganizationCommandHandler {
    async fn handle(
        &self,
        command: CreateOrganizationCommand,
    ) -> Result<OrganizationResponse, CommandError> {
        self.organization_usecase
            .create_organization(&command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("create_failed", &e.to_string()))
    }
}

// =============================================================================
// Get Organization Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct GetOrganizationCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl GetOrganizationCommand {
    pub fn new(organization_id: Uuid, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for GetOrganizationCommand {
    type Result = OrganizationResponse;

    fn command_type(&self) -> &'static str {
        "get_organization"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Organization ID is already validated by UUID type
        Ok(())
    }
}

pub struct GetOrganizationCommandHandler {
    organization_usecase: Arc<dyn OrganizationUseCase>,
}

impl GetOrganizationCommandHandler {
    pub fn new(organization_usecase: Arc<dyn OrganizationUseCase>) -> Self {
        Self {
            organization_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<GetOrganizationCommand> for GetOrganizationCommandHandler {
    async fn handle(
        &self,
        command: GetOrganizationCommand,
    ) -> Result<OrganizationResponse, CommandError> {
        self.organization_usecase
            .get_organization(command.organization_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("get_failed", &e.to_string()))
    }
}

// =============================================================================
// Update Organization Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct UpdateOrganizationCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: UpdateOrganizationRequest,
    pub user_id: Uuid,
}

impl UpdateOrganizationCommand {
    pub fn new(organization_id: Uuid, request: UpdateOrganizationRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for UpdateOrganizationCommand {
    type Result = OrganizationResponse;

    fn command_type(&self) -> &'static str {
        "update_organization"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if let Some(ref name) = self.request.name {
            if name.trim().is_empty() {
                return Err(CommandError::validation(
                    "empty_name",
                    "Organization name cannot be empty",
                ));
            }
        }

        Ok(())
    }
}

pub struct UpdateOrganizationCommandHandler {
    organization_usecase: Arc<dyn OrganizationUseCase>,
}

impl UpdateOrganizationCommandHandler {
    pub fn new(organization_usecase: Arc<dyn OrganizationUseCase>) -> Self {
        Self {
            organization_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<UpdateOrganizationCommand> for UpdateOrganizationCommandHandler {
    async fn handle(
        &self,
        command: UpdateOrganizationCommand,
    ) -> Result<OrganizationResponse, CommandError> {
        self.organization_usecase
            .update_organization(command.organization_id, &command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("update_failed", &e.to_string()))
    }
}

// =============================================================================
// Delete Organization Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct DeleteOrganizationCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
}

impl DeleteOrganizationCommand {
    pub fn new(organization_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for DeleteOrganizationCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "delete_organization"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct DeleteOrganizationCommandHandler {
    organization_usecase: Arc<dyn OrganizationUseCase>,
}

impl DeleteOrganizationCommandHandler {
    pub fn new(organization_usecase: Arc<dyn OrganizationUseCase>) -> Self {
        Self {
            organization_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<DeleteOrganizationCommand> for DeleteOrganizationCommandHandler {
    async fn handle(&self, command: DeleteOrganizationCommand) -> Result<(), CommandError> {
        self.organization_usecase
            .delete_organization(command.organization_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("delete_failed", &e.to_string()))
    }
}

// =============================================================================
// List Organizations Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct ListOrganizationsCommand {
    pub command_id: Uuid,
    pub user_id: Uuid,
    pub pagination: PaginationRequest,
}

impl ListOrganizationsCommand {
    pub fn new(user_id: Uuid, pagination: PaginationRequest) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
            pagination,
        }
    }
}

#[async_trait]
impl Command for ListOrganizationsCommand {
    type Result = OrganizationListResponse;

    fn command_type(&self) -> &'static str {
        "list_organizations"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ListOrganizationsCommandHandler {
    organization_usecase: Arc<dyn OrganizationUseCase>,
}

impl ListOrganizationsCommandHandler {
    pub fn new(organization_usecase: Arc<dyn OrganizationUseCase>) -> Self {
        Self {
            organization_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<ListOrganizationsCommand> for ListOrganizationsCommandHandler {
    async fn handle(
        &self,
        command: ListOrganizationsCommand,
    ) -> Result<OrganizationListResponse, CommandError> {
        self.organization_usecase
            .list_organizations(command.user_id, &command.pagination)
            .await
            .map_err(|e| CommandError::business("list_failed", &e.to_string()))
    }
}

// =============================================================================
// Search Organizations Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct SearchOrganizationsCommand {
    pub command_id: Uuid,
    pub request: OrganizationSearchRequest,
    pub user_id: Option<Uuid>,
}

impl SearchOrganizationsCommand {
    pub fn new(request: OrganizationSearchRequest, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for SearchOrganizationsCommand {
    type Result = OrganizationListResponse;

    fn command_type(&self) -> &'static str {
        "search_organizations"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct SearchOrganizationsCommandHandler {
    organization_usecase: Arc<dyn OrganizationUseCase>,
}

impl SearchOrganizationsCommandHandler {
    pub fn new(organization_usecase: Arc<dyn OrganizationUseCase>) -> Self {
        Self {
            organization_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<SearchOrganizationsCommand> for SearchOrganizationsCommandHandler {
    async fn handle(
        &self,
        command: SearchOrganizationsCommand,
    ) -> Result<OrganizationListResponse, CommandError> {
        self.organization_usecase
            .search_organizations(&command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("search_failed", &e.to_string()))
    }
}

// =============================================================================
// Error Mapper
// =============================================================================

pub struct OrganizationErrorMapper;

impl CommandErrorMapper for OrganizationErrorMapper {
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
            CommandError::business("unknown_error", &error.to_string())
        }
    }
}