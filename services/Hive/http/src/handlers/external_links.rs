use axum::{
    extract::{Path, State},
    response::Json,
};
use hive_application::{
    CreateExternalLinkCommand, CreateExternalLinkRequest, ExternalLinkResponse,
};
use rustycog::command::CommandContext;
use rustycog::http::{AppState, AuthPrincipal, ValidatedJson};
use rustycog::permission::ResourceId;

use crate::error::HttpError;

/// Create an external link
/// POST /api/organizations/{organization_id}/external-links
///
/// # Errors
///
/// Returns [`HttpError`] if command execution fails.
pub async fn create_external_link(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthPrincipal,
    ValidatedJson(request): ValidatedJson<CreateExternalLinkRequest>,
) -> Result<Json<ExternalLinkResponse>, HttpError> {
    tracing::info!(
        "Creating external link for organization: {}",
        organization_id
    );

    let command =
        CreateExternalLinkCommand::new(organization_id.id(), &request, auth_user.principal.subject);
    let context = CommandContext::new()
        .with_user_id(auth_user.principal.subject)
        .with_metadata("principal_issuer".into(), auth_user.principal.issuer);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {e}"),
        })?;

    Ok(Json(result))
}
