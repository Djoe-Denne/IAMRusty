use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        CreateInvitationRequest, InvitationDetailsResponse, InvitationListResponse,
        InvitationResponse, PaginationRequest,
    },
    usecase::InvitationUseCase,
    ApplicationError,
};

#[derive(Debug, Clone)]
pub struct CreateInvitationCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: CreateInvitationRequest,
    pub invited_by_user_id: Uuid,
}

impl CreateInvitationCommand {
    pub fn new(
        organization_id: Uuid,
        request: CreateInvitationRequest,
        invited_by_user_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
            invited_by_user_id,
        }
    }
}

#[async_trait]
impl Command for CreateInvitationCommand {
    type Result = InvitationResponse;

    fn command_type(&self) -> &'static str {
        "create_invitation"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct CreateInvitationCommandHandler {
    invitation_usecase: Arc<dyn InvitationUseCase>,
}

impl CreateInvitationCommandHandler {
    pub fn new(invitation_usecase: Arc<dyn InvitationUseCase>) -> Self {
        Self { invitation_usecase }
    }
}

#[async_trait]
impl CommandHandler<CreateInvitationCommand> for CreateInvitationCommandHandler {
    async fn handle(
        &self,
        command: CreateInvitationCommand,
    ) -> Result<InvitationResponse, CommandError> {
        self.invitation_usecase
            .create_invitation(
                command.organization_id,
                &command.request,
                command.invited_by_user_id,
            )
            .await
            .map_err(|e| CommandError::business("create_invitation_failed", &e.to_string()))
    }
}

// List Invitations Command
#[derive(Debug, Clone)]
pub struct ListInvitationsCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub pagination: PaginationRequest,
    pub user_id: Option<Uuid>,
}

impl ListInvitationsCommand {
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
impl Command for ListInvitationsCommand {
    type Result = InvitationListResponse;

    fn command_type(&self) -> &'static str {
        "list_invitations"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ListInvitationsCommandHandler {
    invitation_usecase: Arc<dyn InvitationUseCase>,
}

impl ListInvitationsCommandHandler {
    pub fn new(invitation_usecase: Arc<dyn InvitationUseCase>) -> Self {
        Self { invitation_usecase }
    }
}

#[async_trait]
impl CommandHandler<ListInvitationsCommand> for ListInvitationsCommandHandler {
    async fn handle(
        &self,
        command: ListInvitationsCommand,
    ) -> Result<InvitationListResponse, CommandError> {
        // TODO: Implement list_invitations in InvitationUseCase
        Err(CommandError::business(
            "list_invitations_not_implemented",
            "List invitations functionality not yet implemented",
        ))
    }
}

// Cancel Invitation Command
#[derive(Debug, Clone)]
pub struct CancelInvitationCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub invitation_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl CancelInvitationCommand {
    pub fn new(organization_id: Uuid, invitation_id: Uuid, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            invitation_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for CancelInvitationCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "cancel_invitation"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct CancelInvitationCommandHandler {
    invitation_usecase: Arc<dyn InvitationUseCase>,
}

impl CancelInvitationCommandHandler {
    pub fn new(invitation_usecase: Arc<dyn InvitationUseCase>) -> Self {
        Self { invitation_usecase }
    }
}

#[async_trait]
impl CommandHandler<CancelInvitationCommand> for CancelInvitationCommandHandler {
    async fn handle(&self, command: CancelInvitationCommand) -> Result<(), CommandError> {
        // TODO: Implement cancel_invitation in InvitationUseCase
        Err(CommandError::business(
            "cancel_invitation_not_implemented",
            "Cancel invitation functionality not yet implemented",
        ))
    }
}

// Accept Invitation Command
#[derive(Debug, Clone)]
pub struct AcceptInvitationCommand {
    pub command_id: Uuid,
    pub token: String,
    pub user_id: Uuid,
}

impl AcceptInvitationCommand {
    pub fn new(token: String, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            token,
            user_id,
        }
    }
}

#[async_trait]
impl Command for AcceptInvitationCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "accept_invitation"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.token.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_token",
                "Invitation token cannot be empty",
            ));
        }
        Ok(())
    }
}

pub struct AcceptInvitationCommandHandler {
    invitation_usecase: Arc<dyn InvitationUseCase>,
}

impl AcceptInvitationCommandHandler {
    pub fn new(invitation_usecase: Arc<dyn InvitationUseCase>) -> Self {
        Self { invitation_usecase }
    }
}

#[async_trait]
impl CommandHandler<AcceptInvitationCommand> for AcceptInvitationCommandHandler {
    async fn handle(&self, command: AcceptInvitationCommand) -> Result<(), CommandError> {
        self.invitation_usecase
            .accept_invitation(command.token, command.user_id)
            .await
            .map_err(|e| CommandError::business("accept_invitation_failed", &e.to_string()))
    }
}

// Get Invitation By Token Command
#[derive(Debug, Clone)]
pub struct GetInvitationByTokenCommand {
    pub command_id: Uuid,
    pub token: String,
}

impl GetInvitationByTokenCommand {
    pub fn new(token: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            token,
        }
    }
}

#[async_trait]
impl Command for GetInvitationByTokenCommand {
    type Result = InvitationDetailsResponse;

    fn command_type(&self) -> &'static str {
        "get_invitation_by_token"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.token.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_token",
                "Invitation token cannot be empty",
            ));
        }
        Ok(())
    }
}

pub struct GetInvitationByTokenCommandHandler {
    invitation_usecase: Arc<dyn InvitationUseCase>,
}

impl GetInvitationByTokenCommandHandler {
    pub fn new(invitation_usecase: Arc<dyn InvitationUseCase>) -> Self {
        Self { invitation_usecase }
    }
}

#[async_trait]
impl CommandHandler<GetInvitationByTokenCommand> for GetInvitationByTokenCommandHandler {
    async fn handle(
        &self,
        command: GetInvitationByTokenCommand,
    ) -> Result<InvitationDetailsResponse, CommandError> {
        // TODO: Implement get_invitation_by_token in InvitationUseCase
        Err(CommandError::business(
            "get_invitation_by_token_not_implemented",
            "Get invitation by token functionality not yet implemented",
        ))
    }
}

// Resend Invitation Command
#[derive(Debug, Clone)]
pub struct ResendInvitationCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub invitation_id: Uuid,
}

impl ResendInvitationCommand {
    pub fn new(organization_id: Uuid, invitation_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            invitation_id,
        }
    }
}

#[async_trait]
impl Command for ResendInvitationCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "resend_invitation"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ResendInvitationCommandHandler {
    invitation_usecase: Arc<dyn InvitationUseCase>,
}

impl ResendInvitationCommandHandler {
    pub fn new(invitation_usecase: Arc<dyn InvitationUseCase>) -> Self {
        Self { invitation_usecase }
    }
}

#[async_trait]
impl CommandHandler<ResendInvitationCommand> for ResendInvitationCommandHandler {
    async fn handle(&self, command: ResendInvitationCommand) -> Result<(), CommandError> {
        // TODO: Implement resend_invitation in InvitationUseCase
        Err(CommandError::business(
            "resend_invitation_not_implemented",
            "Resend invitation functionality not yet implemented",
        ))
    }
}

pub struct InvitationErrorMapper;

impl CommandErrorMapper for InvitationErrorMapper {
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
