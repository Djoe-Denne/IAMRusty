use axum::{
    Json,
    http::HeaderMap,
    extract::{State, Path, Query},
    response::Redirect,
};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use validator::Validate;
use domain::entity::provider::Provider;
use application::command::CommandContext;
use crate::{AppState, oauth_state::OAuthState, error::AuthError, validation::*};
use tracing::{debug, error};
use uuid::Uuid;
use url;

/// OAuth callback query parameters
#[derive(Debug, Deserialize, Validate)]
pub struct OAuthCallbackQuery {
    /// Authorization code from provider
    #[validate(length(max = 1000, message = "Authorization code is too long"))]
    pub code: Option<String>,
    /// State parameter containing operation context
    #[validate(length(max = 2000, message = "State parameter is too long"))]
    pub state: Option<String>,
    /// Error from provider (if any)
    #[validate(length(max = 500, message = "Error message is too long"))]
    pub error: Option<String>,
    /// Error description from provider (if any)
    #[validate(length(max = 1000, message = "Error description is too long"))]
    pub error_description: Option<String>,
}

/// OAuth provider path parameter
#[derive(Debug, Deserialize, Validate)]
pub struct ProviderPath {
    /// Provider name (github, gitlab, etc.)
    #[validate(length(min = 1, max = 50, message = "Provider name must be between 1 and 50 characters"))]
    #[validate(regex(path = "*PROVIDER_REGEX", message = "Provider name can only contain lowercase letters"))]
    #[validate(custom(function = "validate_provider_name", message = "Invalid provider name"))]
    pub provider_name: String,
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
    Valid(Path(provider_path)): Valid<Path<ProviderPath>>,
    headers: HeaderMap,
) -> Result<Redirect, AuthError> {
    debug!("OAuth start for provider: {}", provider_path.provider_name);
    
    // Parse the provider
    let provider = match provider_path.provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err(AuthError::oauth_invalid_provider("start")),
    };
    
    // Check if this is a login or link operation based on Authorization header
    let oauth_state = if let Some(auth_header) = headers.get("Authorization") {
        // Link operation - user is authenticated
        let auth_str = auth_header.to_str()
            .map_err(|_e| AuthError::oauth_invalid_authorization_header("start"))?;
        
        if !auth_str.starts_with("Bearer ") {
            return Err(AuthError::oauth_invalid_authorization_header("start"));
        }
        
        let token = &auth_str[7..];
        
        // Validate the token and get user ID
        let user_id = state.user_usecase
            .validate_token(token)
            .await
            .map_err(|_e| AuthError::oauth_invalid_token("start"))?;
        
        debug!("Creating link state for user: {}", user_id);
        OAuthState::new_link(user_id)
    } else {
        // Login operation - user is not authenticated
        debug!("Creating login state");
        OAuthState::new_login()
    };
    
    // Encode the state
    let encoded_state = oauth_state.encode()
        .map_err(|_e| AuthError::oauth_state_encoding_failed("start"))?;
    
    // Generate provider authorization URL using the command service
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), if oauth_state.is_login() { "login_start".to_string() } else { "link_start".to_string() })
        .with_metadata("provider".to_string(), provider.as_str().to_string());
    
    let base_auth_url = if oauth_state.is_login() {
        // Login operation - use login command
        state.command_service
            .generate_login_start_url(provider, context)
            .await
            .map_err(|_e| AuthError::oauth_url_generation_failed("start"))?
    } else {
        // Link operation - use link provider command
        state.command_service
            .generate_link_provider_start_url(provider, context)
            .await
            .map_err(|_e| AuthError::oauth_url_generation_failed("start"))?
    };
    
    // Parse the URL and add our state parameter
    let mut url = url::Url::parse(&base_auth_url)
        .map_err(|_e| AuthError::oauth_invalid_url("start"))?;
    
    // Add the state parameter to the URL
    url.query_pairs_mut().append_pair("state", &encoded_state);
    
    debug!("Redirecting to provider authorization URL");
    Ok(Redirect::to(url.as_str()))
}

/// Handle OAuth callback - processes both login and link operations
pub async fn oauth_callback(
    State(state): State<AppState>,
    Valid(Path(provider_path)): Valid<Path<ProviderPath>>,
    Valid(Query(query)): Valid<Query<OAuthCallbackQuery>>,
) -> Result<Json<OAuthResponse>, AuthError> {
    debug!("OAuth callback for provider: {}", provider_path.provider_name);
    
    // Check for OAuth errors from provider
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_else(|| "OAuth error".to_string());
        error!("OAuth error from provider: {} - {}", error, description);
        return Err(AuthError::oauth_provider_error("callback", error, description));
    }
    
    // Check if code parameter is present
    let code = match query.code {
        Some(c) if !c.is_empty() => c,
        _ => return Err(AuthError::oauth_missing_code("callback")),
    };
    
    // Parse the provider
    let provider = match provider_path.provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err(AuthError::oauth_invalid_provider("callback")),
    };
    
    // Decode the state to determine operation type
    let oauth_state = if let Some(state_param) = query.state {
        OAuthState::decode(&state_param)
            .map_err(|_e| AuthError::oauth_invalid_state("callback"))?
    } else {
        return Err(AuthError::oauth_missing_state("callback"));
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
        Err(AuthError::oauth_invalid_state_operation("callback"))
    }
}

/// Handle login callback
async fn handle_login_callback(
    state: AppState,
    provider: Provider,
    code: String,
    redirect_uri: String,
) -> Result<Json<OAuthResponse>, AuthError> {
    debug!("Handling login callback");
    
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "login_callback".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());
    
    let response = state
        .command_service
        .login(provider, code, redirect_uri, context)
        .await
        .map_err(|e| {
            error!("Failed to login: {}", e);
            AuthError::oauth_login_failed("login", &e)
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
) -> Result<Json<OAuthResponse>, AuthError> {
    debug!("Handling link callback for user: {}", user_id);
    
    let context = CommandContext::new()
        .with_user_id(user_id)
        .with_metadata("operation".to_string(), "link_callback".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());
    
    let response = state
        .command_service
        .link_provider(user_id, provider, code, redirect_uri, context)
        .await
        .map_err(|e| {
            error!("Failed to link provider: {}", e);
            AuthError::oauth_link_failed("link", &e, provider.as_str())
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