use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use hive_application::{
    AddMemberCommand, AddMemberRequest, GetMemberCommand, ListMembersCommand, MemberListResponse,
    MemberResponse, PaginationRequest, RemoveMemberCommand, UpdateMemberCommand,
    UpdateMemberRequest,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use uuid::Uuid;

use crate::error::HttpError;

/// Add a member to an organization
/// POST /api/organizations/{organization_id}/members
pub async fn add_member(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<AddMemberRequest>,
) -> Result<Json<MemberResponse>, HttpError> {
    tracing::info!("Adding member to organization: {}", organization_id);

    let command = AddMemberCommand::new(organization_id, request, auth_user.user_id);
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

/// List organization members
/// GET /api/organizations/{organization_id}/members
pub async fn list_members(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<MemberListResponse>, HttpError> {
    tracing::info!("Listing members for organization: {}", organization_id);

    let command = ListMembersCommand::new(organization_id, pagination);
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

/// Get a specific member
/// GET /api/organizations/{organization_id}/members/{user_id}
pub async fn get_member(
    State(state): State<AppState>,
    Path((organization_id, user_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
) -> Result<Json<MemberResponse>, HttpError> {
    tracing::info!(
        "Getting member {} from organization: {}",
        user_id,
        organization_id
    );

    let command = GetMemberCommand::new(organization_id, user_id);
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

/// Remove a member from an organization
/// DELETE /api/organizations/{organization_id}/members/{user_id}
pub async fn remove_member(
    State(state): State<AppState>,
    Path((organization_id, user_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Removing member {} from organization: {}",
        user_id,
        organization_id
    );

    let command = RemoveMemberCommand::new(organization_id, user_id, auth_user.user_id);
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

/// Update a member's role
/// PATCH /organizations/{organization_id}/members/{user_id}
pub async fn update_member(
    State(_state): State<AppState>,
    Path((organization_id, user_id)): Path<(Uuid, Uuid)>,
    // TODO: Add update member request DTO
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Updating member {} in organization: {}",
        user_id,
        organization_id
    );

    let command = UpdateMemberCommand::new(organization_id, user_id, request);
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

/// Remove a member from an organization
/// DELETE /organizations/{organization_id}/members/{user_id}
pub async fn remove_member(
    State(_state): State<AppState>,
    Path((organization_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Removing member {} from organization: {}",
        user_id,
        organization_id
    );

    let command = RemoveMemberCommand::new(organization_id, user_id, auth_user.user_id);
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
