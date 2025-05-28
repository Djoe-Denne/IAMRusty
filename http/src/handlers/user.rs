use axum::{
    Json,
    extract::{State, Extension},
};
use serde::Serialize;
use uuid::Uuid;
use crate::error::ApiError;

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
    State(state): State<crate::AppState>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    debug!("Getting user profile for ID: {}", user_id);
    
    // Get the user
    let user = state
        .user_usecase
        .get_user(user_id)
        .await?;
    
    Ok(Json(UserResponse {
        id: user.user.id,
        username: user.user.username,
        email: user.email,
        avatar_url: user.user.avatar_url,
    }))
}

 