use crate::{error::ApiError, validation::*};
use application::command::{token::RefreshTokenCommand, CommandContext};
use axum::{extract::State, Json};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use validator::Validate;

use tracing::debug;

/// Request for token refresh
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenRequest {
    /// The refresh token
    #[validate(custom(
        function = "validate_refresh_token",
        message = "Invalid refresh token format"
    ))]
    pub refresh_token: String,
}

/// Response for token refresh
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// The JWT access token
    #[serde(rename = "access_token")]
    pub token: String,
    /// Access token expiration time in seconds
    pub expires_in: u64,
    /// New refresh token (replaces the old one)
    pub refresh_token: String,
    /// Refresh token expiration time in seconds
    pub refresh_expires_in: u64,
}

/// Handler for refreshing a token
pub async fn refresh_token(
    State(state): State<crate::AppState>,
    Valid(Json(request)): Valid<Json<RefreshTokenRequest>>,
) -> Result<Json<TokenResponse>, ApiError> {
    debug!("Refreshing token");

    let context =
        CommandContext::new().with_metadata("operation".to_string(), "refresh_token".to_string());

    // Refresh the token using command service
    let command = RefreshTokenCommand::new(request.refresh_token);
    let response = state.command_service.execute(command, context).await?;

    Ok(Json(TokenResponse {
        token: response.access_token,
        expires_in: response.expires_in,
        refresh_token: response.refresh_token,
        refresh_expires_in: response.refresh_expires_in,
    }))
}
