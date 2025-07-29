use axum::{
    extract::{Path, State},
    response::Json,
};
use rustycog_http::{AppState, AuthUser, ValidatedJson};
use rustycog_command::CommandContext;
use hive_application::{
    StartSyncJobRequest, SyncJobResponse, SyncJobListResponse,
    SyncJobStatusResponse, SyncJobLogsResponse,
    StartSyncJobCommand,
};
use uuid::Uuid;

use crate::error::HttpError;

/// Start a sync job
/// POST /api/organizations/{organization_id}/sync-jobs
pub async fn start_sync_job(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<StartSyncJobRequest>,
) -> Result<Json<SyncJobResponse>, HttpError> {
    tracing::info!("Starting sync job for organization: {}", organization_id);
    
    let command = StartSyncJobCommand::new(organization_id, request);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| HttpError::Internal {
            message: format!("Command execution failed: {}", e),
        })?;
    
    Ok(Json(result))
} 