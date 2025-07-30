use crate::error::ApiError;
use axum::{extract::State, Json};
use iam_application::command::{user::GetUserCommand, CommandContext};
use rustycog_http::AuthUser;
use serde::Serialize;
use uuid::Uuid;

use tracing::debug;

/// User response
#[derive(Debug, Serialize)]
pub struct UserResponse {
    /// User ID
    pub id: Uuid,
    /// Username
    pub username: String,
    /// Email address
    pub email: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

/// Get the current user's profile
pub async fn get_user(
    State(state): State<rustycog_http::AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserResponse>, ApiError> {
    debug!("Getting user profile for ID: {}", auth_user.user_id);

    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "get_user".to_string());

    // Get the user using command service
    let command = GetUserCommand::new(auth_user.user_id);
    let user = state.command_service.execute(command, context).await?;

    Ok(Json(UserResponse {
        id: user.user.id,
        username: user.user.username.unwrap_or_default(),
        email: user.email,
        avatar_url: user.user.avatar_url,
    }))
}
