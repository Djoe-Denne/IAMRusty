use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use rustycog_command::{Command, CommandHandler, CommandError};

use crate::{
    dto::{
        CreateRoleRequest, UpdateRoleRequest, RoleResponse, RoleListResponse,
        PaginationRequest
    }
};

// Placeholder role use case trait
#[async_trait]
pub trait RoleUseCase: Send + Sync {
    async fn create_role(
        &self,
        organization_id: Uuid,
        request: CreateRoleRequest,
        user_id: Uuid,
    ) -> Result<RoleResponse, crate::ApplicationError>;

    async fn list_roles(
        &self,
        organization_id: Uuid,
        pagination: PaginationRequest,
    ) -> Result<RoleListResponse, crate::ApplicationError>;

    async fn get_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
    ) -> Result<RoleResponse, crate::ApplicationError>;

    async fn update_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
        request: UpdateRoleRequest,
        user_id: Uuid,
    ) -> Result<RoleResponse, crate::ApplicationError>;

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
    pub request: CreateRoleRequest,
    pub user_id: Uuid,
}

impl CreateRoleCommand {
    pub fn new(organization_id: Uuid, request: CreateRoleRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for CreateRoleCommand {
    type Result = RoleResponse;

    fn command_type(&self) -> &'static str {
        "create_role"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.name.trim().is_empty() {
            return Err(CommandError::validation("empty_name", "Role name cannot be empty"));
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
    async fn handle(&self, command: CreateRoleCommand) -> Result<RoleResponse, CommandError> {
        self.role_usecase
            .create_role(command.organization_id, command.request, command.user_id)
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
    type Result = RoleListResponse;

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
    async fn handle(&self, command: ListRolesCommand) -> Result<RoleListResponse, CommandError> {
        self.role_usecase
            .list_roles(command.organization_id, command.pagination)
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
    type Result = RoleResponse;

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
    async fn handle(&self, command: GetRoleCommand) -> Result<RoleResponse, CommandError> {
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
    pub request: UpdateRoleRequest,
    pub user_id: Uuid,
}

impl UpdateRoleCommand {
    pub fn new(organization_id: Uuid, role_id: Uuid, request: UpdateRoleRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            role_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for UpdateRoleCommand {
    type Result = RoleResponse;

    fn command_type(&self) -> &'static str {
        "update_role"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if let Some(ref name) = self.request.name {
            if name.trim().is_empty() {
                return Err(CommandError::validation("empty_name", "Role name cannot be empty"));
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
    async fn handle(&self, command: UpdateRoleCommand) -> Result<RoleResponse, CommandError> {
        self.role_usecase
            .update_role(command.organization_id, command.role_id, command.request, command.user_id)
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

impl RoleErrorMapper {
    pub fn from_application_error(error: crate::ApplicationError) -> CommandError {
        CommandError::business("role_operation_failed", &error.to_string())
    }
} 