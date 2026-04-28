use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use hive_application::{
    GetRoleCommand, ListRolesCommand, MemberRole, MemberRoleListResponse, PaginationRequest,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use rustycog_permission::ResourceId;
use uuid::Uuid;

use crate::error::HttpError;

/// List roles for an organization
/// GET /api/organizations/{organization_id}/roles
pub async fn list_roles(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<MemberRoleListResponse>, HttpError> {
    tracing::info!("Listing roles for organization: {}", organization_id);

    let command = ListRolesCommand::new(organization_id.id(), pagination);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;

    Ok(Json(MemberRoleListResponse { roles: result }))
}

/// Get a specific role
/// GET /api/organizations/{organization_id}/roles/{role_id}
pub async fn get_role(
    State(state): State<AppState>,
    Path((organization_id, role_id)): Path<(ResourceId, ResourceId)>,
    auth_user: AuthUser,
) -> Result<Json<MemberRole>, HttpError> {
    tracing::info!(
        "Getting role {} from organization: {}",
        role_id,
        organization_id
    );

    let command = GetRoleCommand::new(organization_id.id(), role_id.id());
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
