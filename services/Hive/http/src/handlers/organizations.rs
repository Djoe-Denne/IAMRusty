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
use rustycog::command::{CommandContext, CommandError};
use rustycog::http::{AppState, AuthPrincipal, OptionalAuthPrincipal, ValidatedJson};
use rustycog::permission::ResourceId;

use crate::error::HttpError;

fn principal_context(actor: &AuthPrincipal) -> CommandContext {
    CommandContext::new()
        .with_user_id(actor.principal.subject)
        .with_metadata("principal_issuer".into(), actor.principal.issuer.clone())
}

fn error_mapper(error: &CommandError) -> HttpError {
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
        _ => HttpError::Internal {
            message: error.to_string(),
        },
    }
}

/// Create a new organization
/// POST /api/organizations
///
/// # Errors
///
/// Returns [`HttpError`] if the organization cannot be created.
pub async fn create_organization(
    State(state): State<AppState>,
    auth_user: AuthPrincipal,
    ValidatedJson(request): ValidatedJson<CreateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Creating organization: {}", request.name);

    let command = CreateOrganizationCommand::new(
        request,
        auth_user.principal.subject,
        auth_user.principal.issuer.clone(),
    );
    let context = principal_context(&auth_user);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(result))
}

/// Get an organization by ID
/// GET /`api/organizations/{organization_id`}
///
/// # Errors
///
/// Returns [`HttpError`] if the organization cannot be loaded.
pub async fn get_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: OptionalAuthPrincipal,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Getting organization: {}", organization_id);

    let user_id = auth_user
        .principal
        .as_ref()
        .map(|actor| actor.principal.subject);
    let command = GetOrganizationCommand::new(organization_id.id(), user_id);
    let mut context = CommandContext::new();

    if let Some(actor) = &auth_user.principal {
        context = principal_context(actor);
    }

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(result))
}

/// Update an organization
/// PUT /`api/organizations/{organization_id`}
///
/// # Errors
///
/// Returns [`HttpError`] if the organization cannot be updated.
pub async fn update_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthPrincipal,
    ValidatedJson(request): ValidatedJson<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, HttpError> {
    tracing::info!("Updating organization: {}", organization_id);

    let command =
        UpdateOrganizationCommand::new(organization_id.id(), request, auth_user.principal.subject)
            .with_issuer(auth_user.principal.issuer.clone());
    let context = principal_context(&auth_user);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(result))
}

/// Delete an organization
/// DELETE /`api/organizations/{organization_id`}
///
/// # Errors
///
/// Returns [`HttpError`] if the organization cannot be deleted.
pub async fn delete_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthPrincipal,
) -> Result<Json<()>, HttpError> {
    tracing::info!("Deleting organization: {}", organization_id);

    let command = DeleteOrganizationCommand::new(organization_id.id(), auth_user.principal.subject)
        .with_issuer(auth_user.principal.issuer.clone());
    let context = principal_context(&auth_user);

    state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(()))
}

/// List organizations for the current user
/// GET /api/organizations
///
/// # Errors
///
/// Returns [`HttpError`] if the organizations cannot be listed.
pub async fn list_organizations(
    State(state): State<AppState>,
    auth_user: AuthPrincipal,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<OrganizationListResponse>, HttpError> {
    tracing::info!(
        "Listing organizations - page: {:?}, size: {:?}",
        pagination.page,
        pagination.page_size
    );

    let command = ListOrganizationsCommand::new(auth_user.principal.subject, pagination)
        .with_issuer(auth_user.principal.issuer.clone());
    let context = principal_context(&auth_user);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(result))
}

/// Search organizations
/// GET /api/organizations/search
///
/// # Errors
///
/// Returns [`HttpError`] if the search fails.
pub async fn search_organizations(
    State(state): State<AppState>,
    auth_user: OptionalAuthPrincipal,
    Query(search): Query<OrganizationSearchRequest>,
) -> Result<Json<OrganizationListResponse>, HttpError> {
    tracing::info!("Searching organizations with query: {:?}", search.query);

    let user_id = auth_user
        .principal
        .as_ref()
        .map(|actor| actor.principal.subject);
    let mut command = SearchOrganizationsCommand::new(search, user_id);
    let mut context = CommandContext::new();

    if let Some(actor) = &auth_user.principal {
        command = command.with_principal(actor.principal.clone());
        context = principal_context(actor);
    }

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(result))
}
