use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
    role::{CreateMemberRoleRequest, MemberRole, UpdateMemberRoleRequest},
    PaginationRequest,
}, ApplicationError};

// Placeholder role use case trait
#[async_trait]
pub trait RoleUseCase: Send + Sync {
    async fn create_role(
        &self,
        organization_id: Uuid,
        request: &CreateMemberRoleRequest,
        user_id: Uuid,
    ) -> Result<MemberRole, crate::ApplicationError>;

    async fn list_roles(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<Vec<MemberRole>, crate::ApplicationError>;

    async fn get_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
    ) -> Result<MemberRole, crate::ApplicationError>;

    async fn update_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
        request: &UpdateMemberRoleRequest,
        user_id: Uuid,
    ) -> Result<MemberRole, crate::ApplicationError>;

    async fn delete_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), crate::ApplicationError>;
}

// Create Role Command
#[derive(Debug, Clone)]
pub struct CreateRoleCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: CreateMemberRoleRequest,
    pub user_id: Uuid,
}

impl CreateRoleCommand {
    pub fn new(organization_id: Uuid, request: &CreateMemberRoleRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request: request.clone(),
            user_id,
        }
    }
}

#[async_trait]
impl Command for CreateRoleCommand {
    type Result = MemberRole;

    fn command_type(&self) -> &'static str {
        "create_role"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.name.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_name",
                "Role name cannot be empty",
            ));
        }
        Ok(())
    }
}

pub struct CreateRoleCommandHandler {
    role_usecase: Arc<dyn RoleUseCase>,
}

impl CreateRoleCommandHandler {
    pub fn new(role_usecase: Arc<dyn RoleUseCase>) -> Self {
        Self { role_usecase }
    }
}

#[async_trait]
impl CommandHandler<CreateRoleCommand> for CreateRoleCommandHandler {
    async fn handle(&self, command: CreateRoleCommand) -> Result<MemberRole, CommandError> {
        self.role_usecase
            .create_role(command.organization_id, &command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("create_role_failed", &e.to_string()))
    }
}

// List Roles Command
#[derive(Debug, Clone)]
pub struct ListRolesCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub pagination: PaginationRequest,
}

impl ListRolesCommand {
    pub fn new(organization_id: Uuid, pagination: PaginationRequest) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            pagination,
        }
    }
}

#[async_trait]
impl Command for ListRolesCommand {
    type Result = Vec<MemberRole>;

    fn command_type(&self) -> &'static str {
        "list_roles"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ListRolesCommandHandler {
    role_usecase: Arc<dyn RoleUseCase>,
}

impl ListRolesCommandHandler {
    pub fn new(role_usecase: Arc<dyn RoleUseCase>) -> Self {
        Self { role_usecase }
    }
}

#[async_trait]
impl CommandHandler<ListRolesCommand> for ListRolesCommandHandler {
    async fn handle(&self, command: ListRolesCommand) -> Result<Vec<MemberRole>, CommandError> {
        self.role_usecase
            .list_roles(command.organization_id, &command.pagination)
            .await
            .map_err(|e| CommandError::business("list_roles_failed", &e.to_string()))
    }
}

// Get Role Command
#[derive(Debug, Clone)]
pub struct GetRoleCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub role_id: Uuid,
}

impl GetRoleCommand {
    pub fn new(organization_id: Uuid, role_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            role_id,
        }
    }
}

#[async_trait]
impl Command for GetRoleCommand {
    type Result = MemberRole;

    fn command_type(&self) -> &'static str {
        "get_role"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct GetRoleCommandHandler {
    role_usecase: Arc<dyn RoleUseCase>,
}

impl GetRoleCommandHandler {
    pub fn new(role_usecase: Arc<dyn RoleUseCase>) -> Self {
        Self { role_usecase }
    }
}

#[async_trait]
impl CommandHandler<GetRoleCommand> for GetRoleCommandHandler {
    async fn handle(&self, command: GetRoleCommand) -> Result<MemberRole, CommandError> {
        self.role_usecase
            .get_role(command.organization_id, command.role_id)
            .await
            .map_err(|e| CommandError::business("get_role_failed", &e.to_string()))
    }
}

// Update Role Command
#[derive(Debug, Clone)]
pub struct UpdateRoleCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub role_id: Uuid,
    pub request: UpdateMemberRoleRequest,
    pub user_id: Uuid,
}

impl UpdateRoleCommand {
    pub fn new(
        organization_id: Uuid,
        role_id: Uuid,
        request: &UpdateMemberRoleRequest,
        user_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            role_id,
            request: request.clone(),
            user_id,
        }
    }
}

#[async_trait]
impl Command for UpdateRoleCommand {
    type Result = MemberRole;

    fn command_type(&self) -> &'static str {
        "update_role"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if let Some(ref name) = self.request.name {
            if name.trim().is_empty() {
                return Err(CommandError::validation(
                    "empty_name",
                    "Role name cannot be empty",
                ));
            }
        }
        Ok(())
    }
}

pub struct UpdateRoleCommandHandler {
    role_usecase: Arc<dyn RoleUseCase>,
}

impl UpdateRoleCommandHandler {
    pub fn new(role_usecase: Arc<dyn RoleUseCase>) -> Self {
        Self { role_usecase }
    }
}

#[async_trait]
impl CommandHandler<UpdateRoleCommand> for UpdateRoleCommandHandler {
    async fn handle(&self, command: UpdateRoleCommand) -> Result<MemberRole, CommandError> {
        self.role_usecase
            .update_role(
                command.organization_id,
                command.role_id,
                &command.request,
                command.user_id,
            )
            .await
            .map_err(|e| CommandError::business("update_role_failed", &e.to_string()))
    }
}

// Delete Role Command
#[derive(Debug, Clone)]
pub struct DeleteRoleCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub role_id: Uuid,
    pub user_id: Uuid,
}

impl DeleteRoleCommand {
    pub fn new(organization_id: Uuid, role_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            role_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for DeleteRoleCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "delete_role"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct DeleteRoleCommandHandler {
    role_usecase: Arc<dyn RoleUseCase>,
}

impl DeleteRoleCommandHandler {
    pub fn new(role_usecase: Arc<dyn RoleUseCase>) -> Self {
        Self { role_usecase }
    }
}

#[async_trait]
impl CommandHandler<DeleteRoleCommand> for DeleteRoleCommandHandler {
    async fn handle(&self, command: DeleteRoleCommand) -> Result<(), CommandError> {
        self.role_usecase
            .delete_role(command.organization_id, command.role_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("delete_role_failed", &e.to_string()))
    }
}

// Error Mapper
pub struct RoleErrorMapper;

impl CommandErrorMapper for RoleErrorMapper {
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
