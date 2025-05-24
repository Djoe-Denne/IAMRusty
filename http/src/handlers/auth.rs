use axum::{
    Json,
    http::{StatusCode, HeaderMap},
    extract::{State, Path, Query},
    response::Redirect,
};
use serde::{Deserialize, Serialize};
use domain::entity::provider::Provider;
use application::usecase::{login::LoginError, link_provider::LinkProviderError};
use crate::{AppState, oauth_state::OAuthState};
use tracing::{debug, error, warn};
use uuid::Uuid;

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    /// Authorization code from provider
    pub code: String,
    /// State parameter containing operation context
    pub state: Option<String>,
    /// Error from provider (if any)
    pub error: Option<String>,
    /// Error description from provider (if any)
    pub error_description: Option<String>,
}

/// User data for responses
#[derive(Debug, Serialize)]
pub struct UserData {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address (primary email)
    pub email: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

/// Email data for link responses
#[derive(Debug, Serialize)]
pub struct EmailData {
    /// Email ID
    pub id: String,
    /// Email address
    pub email: String,
    /// Whether this is the primary email
    pub is_primary: bool,
    /// Whether this email is verified
    pub is_verified: bool,
}

/// OAuth login response
#[derive(Debug, Serialize)]
pub struct OAuthLoginResponse {
    /// Operation type
    pub operation: String,
    /// User data
    pub user: UserData,
    /// JWT access token
    pub access_token: String,
    /// Access token expiration in seconds
    pub expires_in: u64,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
}

/// OAuth link provider response
#[derive(Debug, Serialize)]
pub struct OAuthLinkResponse {
    /// Operation type
    pub operation: String,
    /// Success message
    pub message: String,
    /// User data
    pub user: UserData,
    /// All user emails
    pub emails: Vec<EmailData>,
    /// Whether a new email was added
    pub new_email_added: bool,
    /// The new email that was added (if any)
    pub new_email: Option<String>,
}

/// OAuth error response
#[derive(Debug, Serialize)]
pub struct OAuthErrorResponse {
    /// Operation type
    pub operation: String,
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
}

/// Combined response type for OAuth callbacks
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum OAuthResponse {
    Login(OAuthLoginResponse),
    Link(OAuthLinkResponse),
}

/// Handle OAuth start - redirects to provider with appropriate state
pub async fn oauth_start(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    headers: HeaderMap,
) -> Result<Redirect, (StatusCode, String)> {
    debug!("OAuth start for provider: {}", provider_name);
    
    // Parse the provider
    let provider = match provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid provider".to_string())),
    };
    
    // Check if this is a login or link operation based on Authorization header
    let oauth_state = if let Some(auth_header) = headers.get("Authorization") {
        // Link operation - user is authenticated
        let auth_str = auth_header.to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Authorization header".to_string()))?;
        
        if !auth_str.starts_with("Bearer ") {
            return Err((StatusCode::BAD_REQUEST, "Authorization header must start with 'Bearer '".to_string()));
        }
        
        let token = &auth_str[7..];
        
        // Validate the token and get user ID
        let user_id = state.user_usecase
            .validate_token(token)
            .await
            .map_err(|e| {
                error!("Failed to validate token: {}", e);
                (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string())
            })?;
        
        debug!("Creating link state for user: {}", user_id);
        OAuthState::new_link(user_id)
    } else {
        // Login operation - user is not authenticated
        debug!("Creating login state");
        OAuthState::new_login()
    };
    
    // Encode the state
    let encoded_state = oauth_state.encode()
        .map_err(|e| {
            error!("Failed to encode OAuth state: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create OAuth state".to_string())
        })?;
    
    // Generate provider authorization URL
    // Note: In a real implementation, you would inject the OAuth client
    // and call its generate_authorize_url method with the state parameter
    let auth_url = format!(
        "https://{}/oauth/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI&state={}",
        match provider {
            Provider::GitHub => "github.com",
            Provider::GitLab => "gitlab.com",
        },
        encoded_state
    );
    
    debug!("Redirecting to provider authorization URL");
    Ok(Redirect::to(&auth_url))
}

/// Handle OAuth callback - processes both login and link operations
pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<OAuthResponse>, (StatusCode, String)> {
    debug!("OAuth callback for provider: {}", provider_name);
    
    // Check for OAuth errors from provider
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_else(|| "OAuth error".to_string());
        error!("OAuth error from provider: {} - {}", error, description);
        return Err((StatusCode::BAD_REQUEST, format!("OAuth error: {}", description)));
    }
    
    // Parse the provider
    let provider = match provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid provider".to_string())),
    };
    
    // Decode the state to determine operation type
    let oauth_state = if let Some(state_param) = query.state {
        OAuthState::decode(&state_param)
            .map_err(|e| {
                error!("Failed to decode OAuth state: {}", e);
                (StatusCode::BAD_REQUEST, "Invalid state parameter".to_string())
            })?
    } else {
        // Default to login if no state (for backward compatibility)
        warn!("No state parameter provided, defaulting to login operation");
        OAuthState::new_login()
    };
    
    // Get redirect URI (in a real implementation, this would be configurable)
    let redirect_uri = format!("https://yourdomain.com/api/auth/{}/callback", provider_name);
    
    if oauth_state.is_login() {
        // Handle login operation
        handle_login_callback(state, provider, query.code, redirect_uri).await
    } else if let Some(user_id) = oauth_state.get_link_user_id() {
        // Handle link operation
        handle_link_callback(state, provider, query.code, redirect_uri, user_id).await
    } else {
        error!("Invalid OAuth state operation");
        Err((StatusCode::BAD_REQUEST, "Invalid operation in state".to_string()))
    }
}

/// Handle login callback
async fn handle_login_callback(
    state: AppState,
    provider: Provider,
    code: String,
    redirect_uri: String,
) -> Result<Json<OAuthResponse>, (StatusCode, String)> {
    debug!("Handling login callback");
    
    let response = state
        .login_usecase
        .login(provider, code, redirect_uri)
        .await
        .map_err(|e| {
            error!("Failed to login: {}", e);
            match e {
                LoginError::AuthError(_) => (StatusCode::UNAUTHORIZED, "Authentication failed".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Login failed".to_string()),
            }
        })?;
    
    Ok(Json(OAuthResponse::Login(OAuthLoginResponse {
        operation: "login".to_string(),
        user: UserData {
            id: response.user.id.to_string(),
            username: response.user.username,
            email: Some(response.email),
            avatar_url: response.user.avatar_url,
        },
        access_token: response.access_token,
        expires_in: response.expires_in,
        refresh_token: response.refresh_token,
    })))
}

/// Handle link provider callback
async fn handle_link_callback(
    state: AppState,
    provider: Provider,
    code: String,
    redirect_uri: String,
    user_id: Uuid,
) -> Result<Json<OAuthResponse>, (StatusCode, String)> {
    debug!("Handling link callback for user: {}", user_id);
    
    let response = state
        .link_provider_usecase
        .link_provider(user_id, provider, code, redirect_uri)
        .await
        .map_err(|e| {
            error!("Failed to link provider: {}", e);
            match e {
                LinkProviderError::ProviderAlreadyLinkedToSameUser => {
                    let error_response = OAuthErrorResponse {
                        operation: "link".to_string(),
                        error: "provider_already_linked_to_same_user".to_string(),
                        message: format!("{} is already linked to your account", provider.as_str()),
                    };
                    return (StatusCode::CONFLICT, serde_json::to_string(&error_response).unwrap_or_default());
                }
                LinkProviderError::ProviderAlreadyLinked => {
                    let error_response = OAuthErrorResponse {
                        operation: "link".to_string(),
                        error: "provider_already_linked".to_string(),
                        message: format!("This {} account is already linked to another user", provider.as_str()),
                    };
                    return (StatusCode::CONFLICT, serde_json::to_string(&error_response).unwrap_or_default());
                }
                LinkProviderError::AuthError(_) => (StatusCode::UNAUTHORIZED, "Authentication failed".to_string()),
                LinkProviderError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Link provider failed".to_string()),
            }
        })?;
    
    // Convert UserEmail entities to EmailData
    let emails: Vec<EmailData> = response.emails
        .into_iter()
        .map(|email| EmailData {
            id: email.id.to_string(),
            email: email.email,
            is_primary: email.is_primary,
            is_verified: email.is_verified,
        })
        .collect();
    
    // Get primary email for user data
    let primary_email = emails
        .iter()
        .find(|e| e.is_primary)
        .map(|e| e.email.clone());
    
    Ok(Json(OAuthResponse::Link(OAuthLinkResponse {
        operation: "link".to_string(),
        message: format!("{} successfully linked", provider.as_str()),
        user: UserData {
            id: response.user.id.to_string(),
            username: response.user.username,
            email: primary_email,
            avatar_url: response.user.avatar_url,
        },
        emails,
        new_email_added: response.new_email_added,
        new_email: response.new_email,
    })))
} 