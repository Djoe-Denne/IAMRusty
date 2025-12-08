use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use manifesto_application::{
    ArchiveProjectCommand, CreateProjectCommand, CreateProjectRequest, DeleteProjectCommand,
    GetProjectCommand, GetProjectDetailCommand, ListProjectsCommand, PaginationRequest,
    ProjectDetailResponse, ProjectListResponse, ProjectResponse, PublishProjectCommand,
    UpdateProjectCommand, UpdateProjectRequest,
};
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser, OptionalAuthUser, ValidatedJson};
use rustycog_permission::ResourceId;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{error_mapper, HttpError};

#[derive(Debug, Deserialize)]
pub struct ProjectQueryParams {
    pub owner_type: Option<String>,
    pub owner_id: Option<Uuid>,
    pub status: Option<String>,
    pub search: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationRequest,
}

/// Create a new project
/// POST /api/projects
pub async fn create_project(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), HttpError> {
    tracing::info!("Creating project: {}", request.name);

    let command = CreateProjectCommand::new(request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// Get a project by ID
/// GET /api/projects/{project_id}
pub async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: OptionalAuthUser,
) -> Result<Json<ProjectResponse>, HttpError> {
    tracing::info!("Getting project: {}", project_id);

    let user_id = auth_user.user_id();
    let command = GetProjectCommand::new(project_id.id(), user_id);
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

/// Get project details (with components and member count)
/// GET /api/projects/{project_id}/details
pub async fn get_project_detail(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: OptionalAuthUser,
) -> Result<Json<ProjectDetailResponse>, HttpError> {
    tracing::info!("Getting project details: {}", project_id);

    let user_id = auth_user.user_id();
    let command = GetProjectDetailCommand::new(project_id.id(), user_id);
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

/// Update a project
/// PUT /api/projects/{project_id}
pub async fn update_project(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, HttpError> {
    tracing::info!("Updating project: {}", project_id);

    let command = UpdateProjectCommand::new(project_id.id(), request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Delete a project
/// DELETE /api/projects/{project_id}
pub async fn delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<(StatusCode, Json<()>), HttpError> {
    tracing::info!("Deleting project: {}", project_id);

    let command = DeleteProjectCommand::new(project_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok((StatusCode::NO_CONTENT, Json(())))
}

/// List projects
/// GET /api/projects
pub async fn list_projects(
    State(state): State<AppState>,
    Query(params): Query<ProjectQueryParams>,
) -> Result<Json<ProjectListResponse>, HttpError> {
    tracing::info!(
        "Listing projects - owner_type: {:?}, owner_id: {:?}, status: {:?}, search: {:?}",
        params.owner_type,
        params.owner_id,
        params.status,
        params.search
    );

    let command = ListProjectsCommand::new(
        params.owner_type,
        params.owner_id,
        params.status,
        params.search,
        params.pagination,
    );
    let context = CommandContext::new();

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Publish a project (transition from draft to active)
/// POST /api/projects/{project_id}/publish
pub async fn publish_project(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<Json<ProjectResponse>, HttpError> {
    tracing::info!("Publishing project: {}", project_id);

    let command = PublishProjectCommand::new(project_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// Archive a project
/// POST /api/projects/{project_id}/archive
pub async fn archive_project(
    State(state): State<AppState>,
    Path(project_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<Json<ProjectResponse>, HttpError> {
    tracing::info!("Archiving project: {}", project_id);

    let command = ArchiveProjectCommand::new(project_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

