use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use rustycog_command::CommandContext;
use hive_application::{
    CreateOrganizationRequest, UpdateOrganizationRequest, OrganizationResponse,
    OrganizationListResponse, OrganizationSearchRequest, PaginationRequest,
    CreateOrganizationCommand, GetOrganizationCommand, UpdateOrganizationCommand,
    DeleteOrganizationCommand, ListOrganizationsCommand, SearchOrganizationsCommand,
};
use uuid::Uuid;

use crate::error::HttpError;

/// Create a new organization
/// POST /api/organizations
pub async fn create_organization(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Creating organization: {}", request.name);
    
    let command = CreateOrganizationCommand::new(request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Get an organization by ID
/// GET /api/organizations/{organization_id}
pub async fn get_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: Option<AuthUser>,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Getting organization: {}", organization_id);
    
    let user_id = auth_user.map(|u| u.user_id);
    let command = GetOrganizationCommand::new(organization_id, user_id);
    let context = CommandContext::new().with_optional_user_id(user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Update an organization
/// PUT /api/organizations/{organization_id}
pub async fn update_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Updating organization: {}", organization_id);
    
    let command = UpdateOrganizationCommand::new(organization_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Delete an organization
/// DELETE /api/organizations/{organization_id}
pub async fn delete_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Deleting organization: {}", organization_id);
    
    let command = DeleteOrganizationCommand::new(organization_id, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// List organizations for the current user
/// GET /api/organizations
pub async fn list_organizations(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<OrganizationListResponse>, HttpError> {
    tracing::info!(
        "Listing organizations - page: {:?}, size: {:?}",
        pagination.page,
        pagination.page_size
    );
    
    let command = ListOrganizationsCommand::new(auth_user.user_id, pagination);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}

/// Search organizations
/// GET /api/organizations/search
pub async fn search_organizations(
    State(state): State<AppState>,
    auth_user: Option<AuthUser>,
    Query(search): Query<OrganizationSearchRequest>,
) -> Result<Json<OrganizationListResponse>, HttpError> {
    tracing::info!("Searching organizations with query: {:?}", search.query);
    
    let user_id = auth_user.map(|u| u.user_id);
    let command = SearchOrganizationsCommand::new(search, user_id);
    let context = CommandContext::new().with_optional_user_id(user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
}