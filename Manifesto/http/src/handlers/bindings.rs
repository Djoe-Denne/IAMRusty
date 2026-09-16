//! Privileged binding grant snapshot HTTP (domain language).

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use manifesto_application::{
    BindingGrantSnapshotResponse, GetBindingGrantSnapshotCommand, UpsertBindingConsentCommand,
    UpsertBindingConsentRequest,
};
use rustycog::command::CommandContext;
use rustycog::http::{AppState, AuthUser, ValidatedJson};
use rustycog::permission::ResourceId;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{error_mapper, HttpError};

/// Optional end-user id for membership intersection. Never taken from the JWT.
#[derive(Debug, Default, Deserialize)]
pub struct BindingGrantSnapshotQuery {
    /// End-user principal to include in the snapshot. Omitted for background callers.
    pub principal: Option<Uuid>,
}

/// GET /`api/projects/{project_id}/bindings/{component_id`}
///
/// Platform-service JWT (`iss=aiforall-platform`, `aud=manifesto-bindings`).
/// Query `principal` is the intersection user when present; the JWT subject is
/// not used as that principal.
///
/// # Errors
///
/// Returns [`HttpError`] if the command fails (missing binding or persistence).
pub async fn get_binding_grant_snapshot(
    State(state): State<AppState>,
    Path((project_id, component_id)): Path<(ResourceId, ResourceId)>,
    Query(query): Query<BindingGrantSnapshotQuery>,
    auth_user: AuthUser,
) -> Result<Json<BindingGrantSnapshotResponse>, HttpError> {
    tracing::info!(
        "Reading binding grant snapshot {} for project {}",
        component_id,
        project_id
    );

    let command =
        GetBindingGrantSnapshotCommand::new(project_id.id(), component_id.id(), query.principal);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}

/// PUT /`api/projects/{project_id}/bindings/{component_id}/consents`
///
/// Authenticated project admin. Upserts a consent row and bumps `grant_revision`
/// in the same transaction.
///
/// # Errors
///
/// Returns [`HttpError`] if the command fails (missing binding, validation, or persistence).
pub async fn upsert_binding_consent(
    State(state): State<AppState>,
    Path((project_id, component_id)): Path<(ResourceId, ResourceId)>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<UpsertBindingConsentRequest>,
) -> Result<Json<BindingGrantSnapshotResponse>, HttpError> {
    tracing::info!(
        "Writing binding consent {} for project {}",
        component_id,
        project_id
    );

    let command = UpsertBindingConsentCommand::new(project_id.id(), component_id.id(), request);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(error_mapper)?;

    Ok(Json(result))
}
