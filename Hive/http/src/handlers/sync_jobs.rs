use axum::{
    extract::{Path, State},
    response::Json,
};
use hive_application::{StartSyncJobCommand, StartSyncJobRequest, SyncJobResponse};
use rustycog::command::{CommandContext, CommandError};
use rustycog::http::{AppState, AuthUser, ValidatedJson};
use uuid::Uuid;

use crate::error::HttpError;

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
        CommandError::Infrastructure { .. } | CommandError::RetryExhausted { .. } | _ => {
            HttpError::Internal {
                message: error.to_string(),
            }
        }
    }
}

/// Start a sync job
/// POST /api/organizations/{organization_id}/sync-jobs
///
/// # Errors
///
/// Returns [`HttpError`] if the sync job command fails validation or execution.
pub async fn start_sync_job(
    State(state): State<AppState>,
    Path(organization_id): Path<Uuid>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<StartSyncJobRequest>,
) -> Result<Json<SyncJobResponse>, HttpError> {
    tracing::info!("Starting sync job for organization: {}", organization_id);

    let command = StartSyncJobCommand::new(organization_id, request, auth_user.user_id);
    let context = CommandContext::new().with_user_id(auth_user.user_id);

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| error_mapper(&e))?;

    Ok(Json(result))
}
