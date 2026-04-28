use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        AddMemberRequest, MemberListResponse, MemberResponse, PaginationRequest,
        UpdateMemberRolesRequest,
    },
    usecase::MemberUseCase,
    ApplicationError,
};

// Add Member Command
#[derive(Debug, Clone)]
pub struct AddMemberCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: AddMemberRequest,
    pub added_by_user_id: Uuid,
}

impl AddMemberCommand {
    pub fn new(organization_id: Uuid, request: &AddMemberRequest, added_by_user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request: request.clone(),
            added_by_user_id,
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
            .add_member(
                command.organization_id,
                &command.request,
                command.added_by_user_id,
            )
            .await
            .map_err(|e| CommandError::business("add_member_failed", &e.to_string()))
    }
}

// Remove Member Command
#[derive(Debug, Clone)]
pub struct RemoveMemberCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub removed_by_user_id: Uuid,
}

impl RemoveMemberCommand {
    pub fn new(organization_id: Uuid, user_id: Uuid, removed_by_user_id: Uuid) -> Self {
        // TODO: Get removed_by_user_id from context
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            user_id,
            removed_by_user_id,
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
            .remove_member(command.organization_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("remove_member_failed", &e.to_string()))
    }
}

// Error Mapper
// List Members Command
#[derive(Debug, Clone)]
pub struct ListMembersCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub pagination: PaginationRequest,
    pub user_id: Option<Uuid>,
}

impl ListMembersCommand {
    pub fn new(
        organization_id: Uuid,
        pagination: PaginationRequest,
        user_id: Option<Uuid>,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            pagination,
            user_id,
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
    async fn handle(
        &self,
        command: ListMembersCommand,
    ) -> Result<MemberListResponse, CommandError> {
        self.member_usecase
            .list_members(command.organization_id, &command.pagination)
            .await
            .map_err(|e| CommandError::business("list_members_failed", &e.to_string()))
    }
}

// Get Member Command
#[derive(Debug, Clone)]
pub struct GetMemberCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub requesting_user_id: Option<Uuid>,
}

impl GetMemberCommand {
    pub fn new(organization_id: Uuid, user_id: Uuid, requesting_user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            user_id,
            requesting_user_id,
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
            .get_member(command.organization_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("get_member_failed", &e.to_string()))
    }
}

// Update Member Command
#[derive(Debug, Clone)]
pub struct UpdateMemberCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub request: UpdateMemberRolesRequest,
    pub requesting_user_id: Uuid,
}

impl UpdateMemberCommand {
    pub fn new(
        organization_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberRolesRequest,
        requesting_user_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            user_id,
            request: request.clone(),
            requesting_user_id,
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
            .update_member(command.organization_id, command.user_id, &command.request)
            .await
            .map_err(|e| CommandError::business("update_member_failed", &e.to_string()))
    }
}

// Error Mapper
pub struct MemberErrorMapper;

impl CommandErrorMapper for MemberErrorMapper {
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
            CommandError::infrastructure("unknown_error", &error.to_string())
        }
    }
}
