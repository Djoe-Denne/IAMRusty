use axum::{
    Json,
    http::StatusCode,
    extract::{State, Extension},
};
use serde::Serialize;
use uuid::Uuid;
use application::usecase::user::{UserUseCase, UserError};
use crate::AppState;
use tracing::{debug, error};

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
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    debug!("Getting user profile for ID: {}", user_id);
    
    // Get the user
    let user = state
        .user_usecase
        .get_user(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user: {}", e);
            match e {
                UserError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user".to_string()),
            }
        })?;
    
    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        avatar_url: user.avatar_url,
    }))
} 