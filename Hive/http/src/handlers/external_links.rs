use axum::{
    extract::{Path, State},
    response::Json,
};
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use rustycog_command::CommandContext;
use hive_application::{
    CreateExternalLinkRequest, UpdateExternalLinkRequest, ToggleSyncRequest,
    ExternalLinkResponse, ExternalLinkListResponse, ConnectionTestResponse,
    CreateExternalLinkCommand,
};
use uuid::Uuid;

use crate::error::HttpError;

/// Create an external link
/// POST /api/organizations/{organization_id}/external-links
pub async fn create_external_link(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<CreateExternalLinkRequest>,
) -> Result<Json<ExternalLinkResponse>, HttpError> {
    tracing::info!("Creating external link for organization: {}", organization_id);
    
    let command = CreateExternalLinkCommand::new(organization_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
} 