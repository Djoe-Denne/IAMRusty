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

fn map_cmd(e: &rustycog::command::CommandError) -> HttpError {
    if let rustycog::command::CommandError::Business { code, .. } = e {
        if code == "iam_signing_invalid_input" {
            return HttpError::BadRequest {
                message: "Invalid signing input".into(),
            };
        }
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

/// POST /`api/organizations/{organization_id}/signer/configure`
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
        .map_err(|e| map_cmd(&e))?;
    Ok(Json(result))
}

/// POST /`api/organizations/{organization_id}/signer/test`
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
        .map_err(|e| map_cmd(&e))?;
    Ok(Json(result))
}

/// POST /`api/organizations/{organization_id}/signer/rotate`
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
        .map_err(|e| map_cmd(&e))?;
    Ok(Json(result))
}

/// POST /`api/organizations/{organization_id}/signer/disable`
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
        .map_err(|e| map_cmd(&e))?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    #[test]
    fn signing_input_is400_epoch409_capacity429_but_provider_and_db_faults_remain500() {
        for (code, status) in [
            ("iam_signing_invalid_input", 400),
            ("iam_signing_epoch_conflict", 409),
            ("iam_signing_admission_throttled", 429),
        ] {
            assert_eq!(
                map_cmd(&rustycog::command::CommandError::business(
                    code,
                    "private-upstream-detail"
                ))
                .into_response()
                .status()
                .as_u16(),
                status
            );
        }
        assert_eq!(
            map_cmd(&rustycog::command::CommandError::infrastructure(
                "provider",
                "private-detail"
            ))
            .into_response()
            .status(),
            500
        );
    }
}
