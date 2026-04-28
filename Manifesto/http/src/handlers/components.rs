use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use manifesto_application::{
    AddComponentCommand, AddComponentRequest, ComponentListResponse, ComponentResponse,
    GetComponentCommand, ListComponentsCommand, RemoveComponentCommand, UpdateComponentRequest,
    UpdateComponentStatusCommand,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, OptionalAuthUser, ValidatedJson};
use rustycog_permission::ResourceId;

use crate::error::{error_mapper, HttpError};

/// Add a component to a project
/// POST /`api/projects/{project_id}/components`
pub async fn add_component(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<AddComponentRequest>,
) -> Result<(StatusCode, Json<ComponentResponse>), HttpError> {
    tracing::info!(
        "Adding component {} to project {}",
        request.component_type,
        project_id
    );

    let command = AddComponentCommand::new(project_id.id(), request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// Get a component by type
/// GET /`api/projects/{project_id}/components/{component_id`}
pub async fn get_component(
    State(state): State<AppState>,
    Path((project_id, component_id)): Path<(ResourceId, ResourceId)>,
    auth_user: OptionalAuthUser,
) -> Result<Json<ComponentResponse>, HttpError> {
    tracing::info!(
        "Getting component {} for project {}",
        component_id,
        project_id
    );

    let user_id = auth_user.user_id();
    let command = GetComponentCommand::new(project_id.id(), component_id.id(), user_id);
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

/// List all components for a project
/// GET /`api/projects/{project_id}/components`
pub async fn list_components(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: OptionalAuthUser,
) -> Result<Json<ComponentListResponse>, HttpError> {
    tracing::info!("Listing components for project {}", project_id);

    let user_id = auth_user.user_id();
    let command = ListComponentsCommand::new(project_id.id(), user_id);
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

/// Update component status
/// PATCH /`api/projects/{project_id}/components/{component_type`}
pub async fn update_component_status(
    State(state): State<AppState>,
    Path((project_id, component_id)): Path<(ResourceId, ResourceId)>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateComponentRequest>,
) -> Result<Json<ComponentResponse>, HttpError> {
    tracing::info!(
        "Updating component {} status for project {}",
        component_id,
        project_id
    );

    let command = UpdateComponentStatusCommand::new(
        project_id.id(),
        component_id.id(),
        request,
        auth_user.user_id,
    );
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Remove a component from a project
/// DELETE /`api/projects/{project_id}/components/{component_type`}
pub async fn remove_component(
    State(state): State<AppState>,
    Path((project_id, component_id)): Path<(ResourceId, ResourceId)>,
    auth_user: AuthUser,
) -> Result<(StatusCode, Json<()>), HttpError> {
    tracing::info!(
        "Removing component {} from project {}",
        component_id,
        project_id
    );

    let command =
        RemoveComponentCommand::new(project_id.id(), component_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::NO_CONTENT, Json(())))
}
