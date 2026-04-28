use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use hive_application::{
    AcceptInvitationCommand, CreateInvitationCommand, CreateInvitationRequest,
    GetInvitationByTokenCommand, InvitationDetailsResponse, InvitationListResponse,
    InvitationResponse, ListInvitationsCommand, PaginationRequest,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, OptionalAuthUser, ValidatedJson};
use rustycog_permission::ResourceId;
use uuid::Uuid;

use crate::error::HttpError;

/// Create an invitation
/// POST /api/organizations/{organization_id}/invitations
pub async fn create_invitation(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateInvitationRequest>,
) -> Result<Json<InvitationResponse>, HttpError> {
    tracing::info!("Creating invitation for organization: {}", organization_id);

    let command = CreateInvitationCommand::new(organization_id.id(), request, auth_user.user_id);
    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "create_invitation".to_string());

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}

/// List organization invitations
/// GET /organizations/{organization_id}/invitations
pub async fn list_invitations(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: OptionalAuthUser,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<InvitationListResponse>, HttpError> {
    tracing::info!("Listing invitations for organization: {}", organization_id);

    let command =
        ListInvitationsCommand::new(organization_id.id(), pagination, auth_user.user_id());
    let mut context = CommandContext::new()
        .with_metadata("operation".to_string(), "list_invitations".to_string());

    if let Some(user_id) = auth_user.user_id() {
        context = context.with_user_id(user_id);
    }

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}

/// Get a specific invitation
/// GET /organizations/{organization_id}/invitations/{invitation_id}
pub async fn get_invitation(
    State(state): State<AppState>,
    Path((_organization_id, invitation_id)): Path<(ResourceId, String)>,
    auth_user: AuthUser,
) -> Result<Json<InvitationDetailsResponse>, HttpError> {
    tracing::info!(
        "Getting invitation {} from organization: {}",
        invitation_id,
        _organization_id
    );

    let command = GetInvitationByTokenCommand::new(invitation_id);
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "get_invitation".to_string())
        .with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}

/// Accept an invitation (public endpoint using token)
/// POST /invitations/{token}/accept
pub async fn accept_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
    auth_user: AuthUser,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Accepting invitation with token: {}", token);

    let command = AcceptInvitationCommand::new(token, auth_user.user_id);
    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "accept_invitation".to_string());

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}

/// Get invitation details by token (public endpoint)
/// GET /invitations/{token}
pub async fn get_invitation_by_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    auth_user: AuthUser,
) -> Result<Json<InvitationDetailsResponse>, HttpError> {
    tracing::info!("Getting invitation details for token: {}", token);

    let command = GetInvitationByTokenCommand::new(token);
    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata(
            "operation".to_string(),
            "get_invitation_by_token".to_string(),
        );

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}
