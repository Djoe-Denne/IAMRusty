use crate::validation::validate_uuid_v4;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_valid::Valid;
use rustycog_command::CommandContext;
use rustycog_http::{AppState, AuthUser};
use serde::Deserialize;
use telegraph_application::command::{
    GetNotificationsCommand, GetUnreadCountCommand, MarkNotificationReadCommand,
};
use uuid::Uuid;
use validator::Validate;

/// Query parameters for getting notifications
#[derive(Debug, Deserialize, Validate)]
pub struct GetNotificationsQuery {
    #[validate(range(min = 0, max = 100))]
    #[serde(default)]
    pub page: Option<u8>,
    #[validate(range(min = 1, max = 100))]
    #[serde(default)]
    pub per_page: Option<u8>,
    #[serde(default)]
    pub unread_only: Option<bool>,
}

/// Request body for marking notification as read (empty, just using path parameter)
#[derive(Debug, Deserialize)]
pub struct MarkNotificationReadRequest {
    // No body needed, we get notification_id from path and user_id from auth
}
// Option 2: Simple struct with custom validation function
#[derive(Debug, Deserialize, Validate)]
pub struct NotificationPathParams {
    #[validate(custom(function = "validate_uuid_v4", message = "Invalid UUID format"))]
    pub id: String,
}

/// Get user notifications with pagination and filtering
/// GET /api/notifications
pub async fn get_notifications(
    auth_user: AuthUser,
    Valid(Query(query)): Valid<Query<GetNotificationsQuery>>,
    State(app_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let command = GetNotificationsCommand::new(
        auth_user.user_id,
        query.page,
        query.per_page,
        query.unread_only,
    );

    let context = CommandContext::new().with_user_id(auth_user.user_id);
    match app_state.command_service.execute(command, context).await {
        Ok(response) => {
            let json_response =
                serde_json::to_value(response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json_response))
        }
        Err(error) => {
            tracing::error!("Failed to get notifications: {:?}", error);
            match &error {
                rustycog_command::CommandError::Validation { .. } => Err(StatusCode::BAD_REQUEST),
                rustycog_command::CommandError::Business { message, .. } => {
                    // Check if it's an unauthorized error
                    if message.contains("Unauthorized") {
                        Err(StatusCode::FORBIDDEN)
                    } else if message.contains("not found") {
                        Err(StatusCode::NOT_FOUND)
                    } else {
                        Err(StatusCode::UNPROCESSABLE_ENTITY)
                    }
                }
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// Get unread notification count for the authenticated user
/// GET /api/notifications/unread-count
pub async fn get_unread_count(
    auth_user: AuthUser,
    State(app_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let command = GetUnreadCountCommand::new(auth_user.user_id);

    let context = CommandContext::new().with_user_id(auth_user.user_id);
    match app_state.command_service.execute(command, context).await {
        Ok(response) => {
            let json_response =
                serde_json::to_value(response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json_response))
        }
        Err(error) => {
            tracing::error!("Failed to get unread count: {:?}", error);
            match &error {
                rustycog_command::CommandError::Validation { .. } => Err(StatusCode::BAD_REQUEST),
                rustycog_command::CommandError::Business { .. } => {
                    Err(StatusCode::UNPROCESSABLE_ENTITY)
                }
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// Mark a notification as read
/// PUT /api/notifications/{id}/read
#[axum::debug_handler]
pub async fn mark_notification_read(
    auth_user: AuthUser,
    Valid(Path(notification_id)): Valid<Path<NotificationPathParams>>,
    State(app_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // UUID is already validated by the path parameter validation
    let uuid: Uuid = notification_id
        .id
        .parse()
        .expect("UUID should be valid after validation");
    let command = MarkNotificationReadCommand::new(uuid, auth_user.user_id);

    let context = CommandContext::new().with_user_id(auth_user.user_id);
    match app_state.command_service.execute(command, context).await {
        Ok(response) => {
            let json_response =
                serde_json::to_value(response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json_response))
        }
        Err(error) => {
            tracing::error!("Failed to mark notification as read: {:?}", error);
            match &error {
                rustycog_command::CommandError::Validation { .. } => Err(StatusCode::BAD_REQUEST),
                rustycog_command::CommandError::Business { message, .. } => {
                    // Check if it's an unauthorized error
                    if message.contains("Unauthorized") {
                        Err(StatusCode::FORBIDDEN)
                    } else if message.contains("not found") {
                        Err(StatusCode::NOT_FOUND)
                    } else {
                        Err(StatusCode::UNPROCESSABLE_ENTITY)
                    }
                }
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}
