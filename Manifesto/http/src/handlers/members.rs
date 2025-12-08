use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use manifesto_application::{
    AddMemberCommand, AddMemberRequest, GetMemberCommand, GrantPermissionCommand,
    GrantPermissionRequest, ListMembersCommand, MemberListResponse, MemberResponse,
    PaginationRequest, RemoveMemberCommand, RevokePermissionCommand,
    UpdateMemberCommand, UpdateMemberPermissionsRequest,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use rustycog_permission::ResourceId;
use uuid::Uuid;

use crate::error::{error_mapper, HttpError};

/// Add a member to a project
/// POST /api/projects/{project_id}/members
pub async fn add_member(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<AddMemberRequest>,
) -> Result<(StatusCode, Json<MemberResponse>), HttpError> {
    tracing::info!("Adding member {} to project {}", request.user_id, project_id);

    let command = AddMemberCommand::new(project_id.id(), request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// Get a member
/// GET /api/projects/{project_id}/members/{user_id}
pub async fn get_member(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(ResourceId, Uuid)>,
    auth_user: AuthUser,
) -> Result<Json<MemberResponse>, HttpError> {
    tracing::info!("Getting member {} for project {}", user_id, project_id);

    let command = GetMemberCommand::new(project_id.id(), user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// List members for a project
/// GET /api/projects/{project_id}/members
pub async fn list_members(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<MemberListResponse>, HttpError> {
    tracing::info!("Listing members for project {}", project_id);

    let command = ListMembersCommand::new(project_id.id(), pagination);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Update a member's permissions
/// PUT /api/projects/{project_id}/members/{user_id}
pub async fn update_member(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(ResourceId, Uuid)>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateMemberPermissionsRequest>,
) -> Result<Json<MemberResponse>, HttpError> {
    tracing::info!(
        "Updating permissions for member {} in project {}",
        user_id,
        project_id
    );

    let command =
        UpdateMemberCommand::new(project_id.id(), user_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Remove a member from a project
/// DELETE /api/projects/{project_id}/members/{user_id}
pub async fn remove_member(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(ResourceId, Uuid)>,
    auth_user: AuthUser,
) -> Result<(StatusCode, Json<()>), HttpError> {
    tracing::info!("Removing member {} from project {}", user_id, project_id);

    let command = RemoveMemberCommand::new(project_id.id(), user_id, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::NO_CONTENT, Json(())))
}

/// Grant a permission to a member
/// POST /api/projects/{project_id}/members/{user_id}/permissions
pub async fn grant_permission(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(ResourceId, Uuid)>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<GrantPermissionRequest>,
) -> Result<Json<MemberResponse>, HttpError> {
    tracing::info!(
        "Granting permission {:?} on resource {:?} to member {} in project {}",
        request.permission,
        request.resource,
        user_id,
        project_id
    );

    let command =
        GrantPermissionCommand::new(project_id.id(), user_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Request query for revoking permissions
#[derive(Debug, serde::Deserialize)]
pub struct RevokePermissionQuery {
    pub resource: String,
}

/// Revoke a permission from a member
/// DELETE /api/projects/{project_id}/members/{user_id}/permissions?resource={resource}
pub async fn revoke_permission(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(ResourceId, Uuid)>,
    Query(query): Query<RevokePermissionQuery>,
    auth_user: AuthUser,
) -> Result<(StatusCode, Json<()>), HttpError> {
    tracing::info!(
        "Revoking permission on resource {} from member {} in project {}",
        query.resource,
        user_id,
        project_id
    );

    let command = RevokePermissionCommand::new(project_id.id(), user_id, query.resource, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::NO_CONTENT, Json(())))
}

