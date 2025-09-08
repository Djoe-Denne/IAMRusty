use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use hive_application::{
    CreateOrganizationCommand, CreateOrganizationRequest, DeleteOrganizationCommand,
    GetOrganizationCommand, ListOrganizationsCommand, OrganizationListResponse,
    OrganizationResponse, OrganizationSearchRequest, PaginationRequest, SearchOrganizationsCommand,
    UpdateOrganizationCommand, UpdateOrganizationRequest,
};
use rustycog_command::{CommandContext, CommandError};
use rustycog_http::{AppState, AuthUser, OptionalAuthUser, ValidatedJson};
use rustycog_permission::ResourceId;

use crate::error::HttpError;


fn error_mapper(error: CommandError) -> HttpError {
    match error {
        CommandError::Validation { .. } => HttpError::Validation {
            message: error.to_string(),
        },
        CommandError::Business { .. } => {
            if error.message().contains("not found") {
                HttpError::NotFound
            } else {
                HttpError::BadRequest {
                    message: error.to_string(),
                }
            }
        }
        CommandError::Infrastructure { .. } => HttpError::Internal {
            message: error.to_string(),
        },
        CommandError::RetryExhausted { .. } => HttpError::Internal {
            message: error.to_string(),
        },
        _ => HttpError::Internal {
            message: error.to_string(),
        },
    }
}

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

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Get an organization by ID
/// GET /api/organizations/{organization_id}
pub async fn get_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: OptionalAuthUser,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Getting organization: {}", organization_id);

    let user_id = auth_user.user_id();
    let command = GetOrganizationCommand::new(organization_id.id(), user_id);
    let mut context = CommandContext::new();

    if let Some(user_id) = user_id {
        context = context.with_user_id(user_id);
    }

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Update an organization
/// PUT /api/organizations/{organization_id}
pub async fn update_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Updating organization: {}", organization_id);

    let command = UpdateOrganizationCommand::new(organization_id.id(), request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Delete an organization
/// DELETE /api/organizations/{organization_id}
pub async fn delete_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Deleting organization: {}", organization_id);

    let command = DeleteOrganizationCommand::new(organization_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

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

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Search organizations
/// GET /api/organizations/search
pub async fn search_organizations(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Query(search): Query<OrganizationSearchRequest>,
) -> Result<Json<OrganizationListResponse>, HttpError> {
    tracing::info!("Searching organizations with query: {:?}", search.query);

    let user_id = auth_user.user_id();
    let command = SearchOrganizationsCommand::new(search, user_id);
    let mut context = CommandContext::new();

    if let Some(user_id) = user_id {
        context = context.with_user_id(user_id);
    }

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}
