use axum::{
    Json,
    http::StatusCode,
    extract::State,
};
use serde::{Deserialize, Serialize};
use application::usecase::token::{TokenUseCase, TokenError};
use crate::AppState;
use tracing::{debug, error};

/// Request for token refresh
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// The refresh token
    pub refresh_token: String,
}

/// Response for token refresh
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// The JWT access token
    pub token: String,
    /// Token expiration time in seconds
    pub expires_in: u64,
}

/// Handler for refreshing a token
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>, (StatusCode, String)> {
    debug!("Refreshing token");
    
    // Refresh the token
    let response = state
        .token_usecase
        .refresh_token(request.refresh_token)
        .await
        .map_err(|e| {
            error!("Failed to refresh token: {}", e);
            match e {
                TokenError::TokenNotFound => (StatusCode::UNAUTHORIZED, "Invalid refresh token".to_string()),
                TokenError::TokenInvalid => (StatusCode::UNAUTHORIZED, "Invalid refresh token".to_string()),
                TokenError::TokenExpired => (StatusCode::UNAUTHORIZED, "Expired refresh token".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to refresh token".to_string()),
            }
        })?;
    
    Ok(Json(TokenResponse {
        token: response.access_token,
        expires_in: response.expires_in,
    }))
} 