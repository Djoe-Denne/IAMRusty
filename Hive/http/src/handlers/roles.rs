use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use rustycog_command::CommandContext;
use hive_application::{
    PaginationRequest, CreateRoleRequest, UpdateRoleRequest,
    RoleResponse, RoleListResponse,
    CreateRoleCommand, ListRolesCommand, GetRoleCommand,
    UpdateRoleCommand, DeleteRoleCommand,
};
use uuid::Uuid;

use crate::error::HttpError;

/// Create a role
/// POST /api/organizations/{organization_id}/roles
pub async fn create_role(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateRoleRequest>,
) -> Result<Json<RoleResponse>, HttpError> {
    tracing::info!("Creating role for organization: {}", organization_id);
    
    let command = CreateRoleCommand::new(organization_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// List roles for an organization
/// GET /api/organizations/{organization_id}/roles
pub async fn list_roles(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<RoleListResponse>, HttpError> {
    tracing::info!("Listing roles for organization: {}", organization_id);
    
    let command = ListRolesCommand::new(organization_id, pagination);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Get a specific role
/// GET /api/organizations/{organization_id}/roles/{role_id}
pub async fn get_role(
    State(state): State<AppState>,
    Path((organization_id, role_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
) -> Result<Json<RoleResponse>, HttpError> {
    tracing::info!(
        "Getting role {} from organization: {}",
        role_id,
        organization_id
    );
    
    let command = GetRoleCommand::new(organization_id, role_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Update a role
/// PUT /api/organizations/{organization_id}/roles/{role_id}
pub async fn update_role(
    State(state): State<AppState>,
    Path((organization_id, role_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, HttpError> {
    tracing::info!(
        "Updating role {} in organization: {}",
        role_id,
        organization_id
    );
    
    let command = UpdateRoleCommand::new(organization_id, role_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Delete a role
/// DELETE /api/organizations/{organization_id}/roles/{role_id}
pub async fn delete_role(
    State(state): State<AppState>,
    Path((organization_id, role_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
) -> Result<Json<()>, HttpError> {
    tracing::info!(
        "Deleting role {} from organization: {}",
        role_id,
        organization_id
    );
    
    let command = DeleteRoleCommand::new(organization_id, role_id, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}