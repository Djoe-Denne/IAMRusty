use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use hive_application::{
    AcceptInvitationCommand, CancelInvitationCommand, CreateInvitationCommand,
    CreateInvitationRequest, GetInvitationByTokenCommand, InvitationDetailsResponse,
    InvitationListResponse, InvitationResponse, ListInvitationsCommand, PaginationRequest,
    ResendInvitationCommand,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use uuid::Uuid;

use crate::error::HttpError;

/// Create an invitation
/// POST /api/organizations/{organization_id}/invitations
pub async fn create_invitation(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateInvitationRequest>,
) -> Result<Json<InvitationResponse>, HttpError> {
    tracing::info!("Creating invitation for organization: {}", organization_id);

    let command = CreateInvitationCommand::new(organization_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

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
    State(_state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Listing invitations for organization: {}", organization_id);

    let command = ListInvitationsCommand::new(organization_id, pagination);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

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
    State(_state): State<AppState>,
    Path((organization_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Getting invitation {} from organization: {}",
        invitation_id,
        organization_id
    );

    let command = CancelInvitationCommand::new(organization_id, invitation_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}

/// Cancel an invitation
/// DELETE /organizations/{organization_id}/invitations/{invitation_id}
pub async fn cancel_invitation(
    State(_state): State<AppState>,
    Path((organization_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Cancelling invitation {} from organization: {}",
        invitation_id,
        organization_id
    );

    Err(HttpError::Internal {
        message: "Not implemented".to_string(),
    })
}

/// Accept an invitation (public endpoint using token)
/// POST /invitations/{token}/accept
pub async fn accept_invitation(
    State(_state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Accepting invitation with token: {}", token);

    let command = AcceptInvitationCommand::new(token);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

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
    State(_state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Getting invitation details for token: {}", token);

    let command = GetInvitationByTokenCommand::new(token);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}

/// Resend an invitation
/// POST /organizations/{organization_id}/invitations/{invitation_id}/resend
pub async fn resend_invitation(
    State(_state): State<AppState>,
    Path((organization_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Resending invitation {} from organization: {}",
        invitation_id,
        organization_id
    );

    let command = ResendInvitationCommand::new(organization_id, invitation_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(result))
}
