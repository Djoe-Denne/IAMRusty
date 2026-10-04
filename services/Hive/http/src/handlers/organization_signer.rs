//! Hive admin HTTP for IAM organization-signer RPC (ADR-0306).

use axum::{
    extract::{Path, State},
    Json,
};
use hive_application::{
    ConfigureOrganizationSignerCommand, ConfigureOrganizationSignerHttpRequest,
    DisableOrganizationSignerCommand, OrganizationSignerHttpResponse,
    RotateOrganizationSignerCommand, TestOrganizationSignerCommand,
};
use rustycog::command::CommandContext;
use rustycog::http::{AppState, AuthUser};
use rustycog::permission::ResourceId;

use crate::error::HttpError;

fn map_cmd(e: rustycog::command::CommandError) -> HttpError {
    if let rustycog::command::CommandError::Business { code, .. } = &e {
        if code == "iam_signing_admission_throttled" {
            return HttpError::RateLimit;
        }
        if code == "iam_signing_epoch_conflict" {
            return HttpError::Conflict {
                message: "Signing epoch conflict".into(),
            };
        }
    }
    HttpError::Internal {
        message: format!("Command execution failed: {e}"),
    }
}

/// POST /api/organizations/{organization_id}/signer/configure
///
/// # Errors
///
/// Returns [`HttpError`] if command execution fails.
pub async fn configure_organization_signer(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
    Json(request): Json<ConfigureOrganizationSignerHttpRequest>,
) -> Result<Json<OrganizationSignerHttpResponse>, HttpError> {
    let command =
        ConfigureOrganizationSignerCommand::new(organization_id.id(), request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(map_cmd)?;
    Ok(Json(result))
}

/// POST /api/organizations/{organization_id}/signer/test
///
/// # Errors
///
/// Returns [`HttpError`] if command execution fails.
pub async fn test_organization_signer(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<Json<OrganizationSignerHttpResponse>, HttpError> {
    let command = TestOrganizationSignerCommand::new(organization_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(map_cmd)?;
    Ok(Json(result))
}

/// POST /api/organizations/{organization_id}/signer/rotate
///
/// # Errors
///
/// Returns [`HttpError`] if command execution fails.
pub async fn rotate_organization_signer(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<Json<OrganizationSignerHttpResponse>, HttpError> {
    let command = RotateOrganizationSignerCommand::new(organization_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(map_cmd)?;
    Ok(Json(result))
}

/// POST /api/organizations/{organization_id}/signer/disable
///
/// # Errors
///
/// Returns [`HttpError`] if command execution fails.
pub async fn disable_organization_signer(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> Result<Json<OrganizationSignerHttpResponse>, HttpError> {
    let command = DisableOrganizationSignerCommand::new(organization_id.id(), auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(map_cmd)?;
    Ok(Json(result))
}
