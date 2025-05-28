use axum::{
    Json,
    extract::State,
};
use serde::{Deserialize, Serialize};
use crate::error::ApiError;

use tracing::debug;

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
    State(state): State<crate::AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    debug!("Refreshing token");
    
    // Refresh the token
    let response = state
        .token_usecase
        .refresh_token(request.refresh_token)
        .await?;
    
    Ok(Json(TokenResponse {
        token: response.access_token,
        expires_in: response.expires_in,
    }))
} 