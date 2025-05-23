use axum::{
    Json,
    http::StatusCode,
    extract::{State, Path},
};
use serde::{Deserialize, Serialize};
use domain::entity::provider::Provider;
use application::usecase::login::{LoginUseCase, LoginError};
use crate::AppState;
use tracing::{debug, error};

/// OAuth login request
#[derive(Debug, Deserialize)]
pub struct OAuthLoginRequest {
    /// Authorization code
    pub code: String,
    /// Redirect URI
    pub redirect_uri: String,
}

/// OAuth login response
#[derive(Debug, Serialize)]
pub struct OAuthLoginResponse {
    /// User data
    pub user: UserData,
    /// JWT access token
    pub access_token: String,
    /// Access token expiration in seconds
    pub expires_in: u64,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
}

/// User data
#[derive(Debug, Serialize)]
pub struct UserData {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

/// Handle OAuth login
pub async fn oauth_login(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    Json(request): Json<OAuthLoginRequest>,
) -> Result<Json<OAuthLoginResponse>, (StatusCode, String)> {
    debug!("OAuth login for provider: {}", provider_name);
    
    // Parse the provider
    let provider = match provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid provider".to_string())),
    };
    
    // Login the user
    let response = state
        .login_usecase
        .login(provider, request.code, request.redirect_uri)
        .await
        .map_err(|e| {
            error!("Failed to login: {}", e);
            match e {
                LoginError::AuthError(_) => (StatusCode::UNAUTHORIZED, "Authentication failed".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Login failed".to_string()),
            }
        })?;
    
    Ok(Json(OAuthLoginResponse {
        user: UserData {
            id: response.user.id.to_string(),
            username: response.user.username,
            email: response.user.email,
            avatar_url: response.user.avatar_url,
        },
        access_token: response.access_token,
        expires_in: response.expires_in,
        refresh_token: response.refresh_token,
    }))
}

/// Handle OAuth callback
pub async fn oauth_callback() -> Result<String, (StatusCode, String)> {
    // This endpoint is for informational purposes only
    // The actual OAuth callback is handled by the frontend
    Ok("OAuth callback received. Please close this window and return to the application.".to_string())
} 