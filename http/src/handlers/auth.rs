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
use url;

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    /// Authorization code from provider
    pub code: Option<String>,
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
) -> Result<Redirect, (StatusCode, Json<OAuthErrorResponse>)> {
    debug!("OAuth start for provider: {}", provider_name);
    
    // Parse the provider
    let provider = match provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
            operation: "start".to_string(),
            error: "invalid_provider".to_string(),
            message: "Invalid provider".to_string(),
        }))),
    };
    
    // Check if this is a login or link operation based on Authorization header
    let oauth_state = if let Some(auth_header) = headers.get("Authorization") {
        // Link operation - user is authenticated
        let auth_str = auth_header.to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
                operation: "start".to_string(),
                error: "invalid_authorization_header".to_string(),
                message: "Invalid Authorization header".to_string(),
            })))?;
        
        if !auth_str.starts_with("Bearer ") {
            return Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
                operation: "start".to_string(),
                error: "invalid_authorization_header".to_string(),
                message: "Authorization header must start with 'Bearer '".to_string(),
            })));
        }
        
        let token = &auth_str[7..];
        
        // Validate the token and get user ID
        let user_id = state.user_usecase
            .validate_token(token)
            .await
            .map_err(|e| {
                error!("Failed to validate token: {}", e);
                (StatusCode::UNAUTHORIZED, Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "invalid_token".to_string(),
                    message: "Invalid or expired token".to_string(),
                }))
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
            (StatusCode::INTERNAL_SERVER_ERROR, Json(OAuthErrorResponse {
                operation: "start".to_string(),
                error: "state_encoding_failed".to_string(),
                message: "Failed to create OAuth state".to_string(),
            }))
        })?;
    
    // Generate provider authorization URL using the real OAuth clients
    let base_auth_url = if oauth_state.is_login() {
        // Login operation - use login use case
        state.login_usecase
            .generate_start_url(provider)
            .map_err(|e| {
                error!("Failed to generate login authorization URL: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "url_generation_failed".to_string(),
                    message: "Failed to generate authorization URL".to_string(),
                }))
            })?
    } else {
        // Link operation - use link provider use case
        state.link_provider_usecase
            .generate_start_url(provider)
            .map_err(|e| {
                error!("Failed to generate link authorization URL: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "url_generation_failed".to_string(),
                    message: "Failed to generate authorization URL".to_string(),
                }))
            })?
    };
    
    // Parse the URL and add our state parameter
    let mut url = url::Url::parse(&base_auth_url)
        .map_err(|e| {
            error!("Failed to parse authorization URL: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(OAuthErrorResponse {
                operation: "start".to_string(),
                error: "invalid_url".to_string(),
                message: "Invalid authorization URL".to_string(),
            }))
        })?;
    
    // Add the state parameter to the URL
    url.query_pairs_mut().append_pair("state", &encoded_state);
    
    debug!("Redirecting to provider authorization URL");
    Ok(Redirect::to(url.as_str()))
}

/// Handle OAuth callback - processes both login and link operations
pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<OAuthResponse>, (StatusCode, Json<OAuthErrorResponse>)> {
    debug!("OAuth callback for provider: {}", provider_name);
    
    // Check for OAuth errors from provider
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_else(|| "OAuth error".to_string());
        error!("OAuth error from provider: {} - {}", error, description);
        return Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
            operation: "callback".to_string(),
            error: error,
            message: description,
        })));
    }
    
    // Check if code parameter is present
    let code = match query.code {
        Some(c) if !c.is_empty() => c,
        _ => {
            return Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
                operation: "callback".to_string(),
                error: "missing_code".to_string(),
                message: "Missing code parameter".to_string(),
            })));
        }
    };
    
    // Parse the provider
    let provider = match provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
            operation: "callback".to_string(),
            error: "invalid_provider".to_string(),
            message: "Invalid provider".to_string(),
        }))),
    };
    
    // Decode the state to determine operation type
    let oauth_state = if let Some(state_param) = query.state {
        OAuthState::decode(&state_param)
            .map_err(|e| {
                error!("Failed to decode OAuth state: {}", e);
                (StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
                    operation: "callback".to_string(),
                    error: "invalid_state".to_string(),
                    message: "Invalid state parameter".to_string(),
                }))
            })?
    } else {
        return Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
            operation: "callback".to_string(),
            error: "missing_state".to_string(),
            message: "Missing state parameter".to_string(),
        })));
    };
    
    // Get redirect URI from configuration instead of hardcoding
    let redirect_uri = match provider {
        Provider::GitHub => &state.oauth_config.github.redirect_uri,
        Provider::GitLab => &state.oauth_config.gitlab.redirect_uri,
    }.clone();
    
    if oauth_state.is_login() {
        // Handle login operation
        handle_login_callback(state, provider, code, redirect_uri).await
    } else if let Some(user_id) = oauth_state.get_link_user_id() {
        // Handle link operation
        println!("handle_link_callback {:?}, {:?}, {:?}", user_id, code, redirect_uri);
        handle_link_callback(state, provider, code, redirect_uri, user_id).await
    } else {
        error!("Invalid OAuth state operation");
        Err((StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
            operation: "callback".to_string(),
            error: "invalid_state_operation".to_string(),
            message: "Invalid operation in state".to_string(),
        })))
    }
}

/// Handle login callback
async fn handle_login_callback(
    state: AppState,
    provider: Provider,
    code: String,
    redirect_uri: String,
) -> Result<Json<OAuthResponse>, (StatusCode, Json<OAuthErrorResponse>)> {
    debug!("Handling login callback");
    
    let response = state
        .login_usecase
        .login(provider, code, redirect_uri)
        .await
        .map_err(|e| {
            error!("Failed to login: {}", e);
            match e {
                LoginError::AuthError(_) => (StatusCode::UNAUTHORIZED, Json(OAuthErrorResponse {
                    operation: "login".to_string(),
                    error: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                })),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(OAuthErrorResponse {
                    operation: "login".to_string(),
                    error: "login_failed".to_string(),
                    message: "Login failed".to_string(),
                })),
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
) -> Result<Json<OAuthResponse>, (StatusCode, Json<OAuthErrorResponse>)> {
    debug!("Handling link callback for user: {}", user_id);
    
    let response = state
        .link_provider_usecase
        .link_provider(user_id, provider, code, redirect_uri)
        .await
        .map_err(|e| {
            error!("Failed to link provider: {}", e);
            match e {
                LinkProviderError::ProviderAlreadyLinkedToSameUser => {
                    (StatusCode::CONFLICT, Json(OAuthErrorResponse {
                        operation: "link".to_string(),
                        error: "provider_already_linked_to_same_user".to_string(),
                        message: format!("{} is already linked to your account", provider.as_str()),
                    }))
                }
                LinkProviderError::ProviderAlreadyLinked => {
                    (StatusCode::CONFLICT, Json(OAuthErrorResponse {
                        operation: "link".to_string(),
                        error: "provider_already_linked".to_string(),
                        message: format!("This {} account is already linked to another user", provider.as_str()),
                    }))
                }
                LinkProviderError::AuthError(_) => (StatusCode::UNAUTHORIZED, Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                })),
                LinkProviderError::UserNotFound => (StatusCode::NOT_FOUND, Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "user_not_found".to_string(),
                    message: "User not found".to_string(),
                })),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "link_failed".to_string(),
                    message: "Link provider failed".to_string(),
                })),
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