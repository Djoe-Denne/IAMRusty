use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        AddMemberRequest, GrantPermissionRequest, MemberListResponse, MemberResponse,
        PaginationRequest, UpdateMemberPermissionsRequest,
    },
    usecase::MemberUseCase,
    ApplicationError,
};

// Type alias for UpdateMemberRequest
pub type UpdateMemberRequest = UpdateMemberPermissionsRequest;

// =============================================================================
// Error Mapper
// =============================================================================

pub struct MemberErrorMapper;

impl CommandErrorMapper for MemberErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(error) = error.downcast_ref::<ApplicationError>() {
            match error {
                ApplicationError::Domain(domain_error) => {
                    CommandError::business("domain_error", &domain_error.to_string())
                }
                ApplicationError::Validation(msg) => {
                    CommandError::validation("validation_failed", msg)
                }
                ApplicationError::NotFound(msg) => {
                    CommandError::business("not_found", msg)
                }
                ApplicationError::AlreadyExists(msg) => {
                    CommandError::business("already_exists", msg)
                }
                ApplicationError::Internal(msg) => {
                    CommandError::infrastructure("internal_error", msg)
                }
            }
        } else {
            CommandError::business("unknown_error", &error.to_string())
        }
    }
}

// =============================================================================
// Add Member Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct AddMemberCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub request: AddMemberRequest,
    pub added_by: Uuid,
}

impl AddMemberCommand {
    pub fn new(project_id: Uuid, request: AddMemberRequest, added_by: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            request,
            added_by,
        }
    }
}

#[async_trait]
impl Command for AddMemberCommand {
    type Result = MemberResponse;

    fn command_type(&self) -> &'static str {
        "add_member"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.permission.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_permission",
                "Member permission cannot be empty",
            ));
        }
        
        // Validate permission level
        let valid_permissions = ["read", "write", "admin", "owner"];
        if !valid_permissions.contains(&self.request.permission.to_lowercase().as_str()) {
            return Err(CommandError::validation(
                "invalid_permission",
                "Permission must be one of: read, write, admin, owner",
            ));
        }
        
        Ok(())
    }
}

pub struct AddMemberCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl AddMemberCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<AddMemberCommand> for AddMemberCommandHandler {
    async fn handle(&self, command: AddMemberCommand) -> Result<MemberResponse, CommandError> {
        self.member_usecase
            .add_member(command.project_id, &command.request, command.added_by)
            .await
            .map_err(|e| CommandError::business("add_failed", &e.to_string()))
    }
}

// =============================================================================
// Get Member Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct GetMemberCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
}

impl GetMemberCommand {
    pub fn new(project_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for GetMemberCommand {
    type Result = MemberResponse;

    fn command_type(&self) -> &'static str {
        "get_member"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct GetMemberCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl GetMemberCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<GetMemberCommand> for GetMemberCommandHandler {
    async fn handle(&self, command: GetMemberCommand) -> Result<MemberResponse, CommandError> {
        self.member_usecase
            .get_member(command.project_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("get_failed", &e.to_string()))
    }
}

// =============================================================================
// List Members Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct ListMembersCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub pagination: PaginationRequest,
}

impl ListMembersCommand {
    pub fn new(project_id: Uuid, pagination: PaginationRequest) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            pagination,
        }
    }
}

#[async_trait]
impl Command for ListMembersCommand {
    type Result = MemberListResponse;

    fn command_type(&self) -> &'static str {
        "list_members"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ListMembersCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl ListMembersCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<ListMembersCommand> for ListMembersCommandHandler {
    async fn handle(&self, command: ListMembersCommand) -> Result<MemberListResponse, CommandError> {
        self.member_usecase
            .list_members(command.project_id, &command.pagination)
            .await
            .map_err(|e| CommandError::business("list_failed", &e.to_string()))
    }
}

// =============================================================================
// Update Member Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct UpdateMemberCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub request: UpdateMemberRequest,
    pub requester_id: Uuid,
}

impl UpdateMemberCommand {
    pub fn new(
        project_id: Uuid,
        user_id: Uuid,
        request: UpdateMemberRequest,
        requester_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
            request,
            requester_id,
        }
    }
}

#[async_trait]
impl Command for UpdateMemberCommand {
    type Result = MemberResponse;

    fn command_type(&self) -> &'static str {
        "update_member"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.permissions.is_empty() {
            return Err(CommandError::validation(
                "empty_permissions",
                "At least one permission must be specified",
            ));
        }
        
        // Validate each permission
        let valid_permissions = ["read", "write", "admin", "owner"];
        for perm in &self.request.permissions {
            if perm.resource.trim().is_empty() {
                return Err(CommandError::validation(
                    "empty_resource",
                    "Resource name cannot be empty",
                ));
            }
            if perm.permission.trim().is_empty() {
                return Err(CommandError::validation(
                    "empty_permission",
                    "Permission level cannot be empty",
                ));
            }
            if !valid_permissions.contains(&perm.permission.to_lowercase().as_str()) {
                return Err(CommandError::validation(
                    "invalid_permission",
                    "Permission must be one of: read, write, admin, owner",
                ));
            }
        }
        
        Ok(())
    }
}

pub struct UpdateMemberCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl UpdateMemberCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<UpdateMemberCommand> for UpdateMemberCommandHandler {
    async fn handle(&self, command: UpdateMemberCommand) -> Result<MemberResponse, CommandError> {
        self.member_usecase
            .update_member(
                command.project_id,
                command.user_id,
                &command.request,
                command.requester_id,
            )
            .await
            .map_err(|e| CommandError::business("update_failed", &e.to_string()))
    }
}

// =============================================================================
// Remove Member Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct RemoveMemberCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub requester_id: Uuid,
}

impl RemoveMemberCommand {
    pub fn new(project_id: Uuid, user_id: Uuid, requester_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
            requester_id,
        }
    }
}

#[async_trait]
impl Command for RemoveMemberCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "remove_member"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct RemoveMemberCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl RemoveMemberCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<RemoveMemberCommand> for RemoveMemberCommandHandler {
    async fn handle(&self, command: RemoveMemberCommand) -> Result<(), CommandError> {
        self.member_usecase
            .remove_member(command.project_id, command.user_id, command.requester_id)
            .await
            .map_err(|e| CommandError::business("remove_failed", &e.to_string()))
    }
}

// =============================================================================
// Grant Permission Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct GrantPermissionCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub request: GrantPermissionRequest,
    pub requester_id: Uuid,
}

impl GrantPermissionCommand {
    pub fn new(
        project_id: Uuid,
        user_id: Uuid,
        request: GrantPermissionRequest,
        requester_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
            request,
            requester_id,
        }
    }
}

#[async_trait]
impl Command for GrantPermissionCommand {
    type Result = MemberResponse;

    fn command_type(&self) -> &'static str {
        "grant_permission"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.resource.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_resource",
                "Resource name cannot be empty",
            ));
        }
        
        if self.request.permission.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_permission",
                "Permission level cannot be empty",
            ));
        }
        
        let valid_permissions = ["read", "write", "admin", "owner"];
        if !valid_permissions.contains(&self.request.permission.to_lowercase().as_str()) {
            return Err(CommandError::validation(
                "invalid_permission",
                "Permission must be one of: read, write, admin, owner",
            ));
        }
        
        Ok(())
    }
}

pub struct GrantPermissionCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl GrantPermissionCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<GrantPermissionCommand> for GrantPermissionCommandHandler {
    async fn handle(&self, command: GrantPermissionCommand) -> Result<MemberResponse, CommandError> {
        self.member_usecase
            .grant_permission(
                command.project_id,
                command.user_id,
                &command.request,
                command.requester_id,
            )
            .await
            .map_err(|e| CommandError::business("grant_failed", &e.to_string()))
    }
}

// =============================================================================
// Revoke Permission Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct RevokePermissionCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub resource: String,
    pub requester_id: Uuid,
}

impl RevokePermissionCommand {
    pub fn new(
        project_id: Uuid,
        user_id: Uuid,
        resource: String,
        requester_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
            resource,
            requester_id,
        }
    }
}

#[async_trait]
impl Command for RevokePermissionCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "revoke_permission"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.resource.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_resource",
                "Resource name cannot be empty",
            ));
        }
        Ok(())
    }
}

pub struct RevokePermissionCommandHandler {
    member_usecase: Arc<dyn MemberUseCase>,
}

impl RevokePermissionCommandHandler {
    pub fn new(member_usecase: Arc<dyn MemberUseCase>) -> Self {
        Self { member_usecase }
    }
}

#[async_trait]
impl CommandHandler<RevokePermissionCommand> for RevokePermissionCommandHandler {
    async fn handle(&self, command: RevokePermissionCommand) -> Result<(), CommandError> {
        self.member_usecase
            .revoke_permission(
                command.project_id,
                command.user_id,
                &command.resource,
                command.requester_id,
            )
            .await
            .map_err(|e| CommandError::business("revoke_failed", &e.to_string()))
    }
}

