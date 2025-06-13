use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    extract::{State, Path, Query},
    response::Redirect,
};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use validator::Validate;
use domain::entity::provider::Provider;
use application::command::{
    CommandContext,
    oauth_login::{OAuthLoginCommand, GenerateOAuthStartUrlCommand},
    provider::{LinkProviderCommand, GenerateLinkProviderStartUrlCommand, GetProviderTokenCommand},
    signup::SignupCommand,
    password_login::PasswordLoginCommand,
    verify_email::VerifyEmailCommand,
    resend_verification_email::ResendVerificationEmailCommand,
};
use crate::{AppState, oauth_state::OAuthState, error::AuthError, validation::*, extractors::ValidatedJson, middleware_auth::AuthUser};
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
    #[validate(regex(path = "*PROVIDER_REGEX", message = "Provider name can only contain letters"))]
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

/// Email/password signup request
#[derive(Debug, Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(custom(function = "crate::validation::validate_username", message = "Username must be 3-30 characters and contain only letters, numbers, underscores, and hyphens"))]
    pub username: String,
    #[validate(custom(function = "crate::validation::validate_email_format", message = "Invalid email format"))]
    pub email: String,
    #[validate(custom(function = "crate::validation::validate_strong_password", message = "Password must be at least 8 characters and contain both letters and numbers"))]
    pub password: String,
}

/// Email/password login request
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(custom(function = "crate::validation::validate_email_format", message = "Invalid email format"))]
    pub email: String,
    #[validate(custom(function = "crate::validation::validate_non_empty_string", message = "Password is required"))]
    pub password: String,
}

/// Email verification request
#[derive(Debug, Deserialize, Validate)]
pub struct VerifyEmailRequest {
    #[validate(custom(function = "crate::validation::validate_email_format", message = "Invalid email format"))]
    pub email: String,
    #[validate(custom(function = "crate::validation::validate_verification_token", message = "Invalid verification token format"))]
    pub verification_token: String,
}

/// Resend verification email request
#[derive(Debug, Deserialize, Validate)]
pub struct ResendVerificationEmailRequest {
    #[validate(custom(function = "crate::validation::validate_email_format", message = "Invalid email format"))]
    pub email: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// Email/password login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserData,
    pub token: String,
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
        let command = GenerateOAuthStartUrlCommand::new(provider);
        state.command_service
            .execute(command, context)
            .await
            .map_err(|_e| AuthError::oauth_url_generation_failed("start"))?
    } else {
        // Link operation - use link provider command
        let command = GenerateLinkProviderStartUrlCommand::new(provider);
        state.command_service
            .execute(command, context)
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
        debug!("handle_link_callback {:?}, {:?}, {:?}", user_id, code, redirect_uri);
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
    
    let command = OAuthLoginCommand::new(provider, code, redirect_uri);
    let response = state
        .command_service
        .execute(command, context)
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
    
    let command = LinkProviderCommand::new(user_id, provider, code, redirect_uri);
    let response = state
        .command_service
        .execute(command, context)
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

/// Handle email/password signup
pub async fn signup(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<SignupRequest>,
) -> Result<(StatusCode, Json<SuccessResponse>), AuthError> {
    debug!("Email/password signup for email: {}", request.email);
    
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "signup".to_string())
        .with_metadata("email".to_string(), request.email.clone());
    
    let command = SignupCommand::new(request.username, request.email, request.password);
    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Signup failed: {}", e);
            AuthError::signup_failed(&e)
        })?;
    
    Ok((StatusCode::CREATED, Json(SuccessResponse {
        message: response.message,
    })))
}

/// Handle email/password login
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    debug!("Email/password login for email: {}", request.email);
    
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "login".to_string())
        .with_metadata("email".to_string(), request.email.clone());
    
    let command = PasswordLoginCommand::new(request.email, request.password);
    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Login failed: {}", e);
            AuthError::login_failed(&e)
        })?;
    
    Ok(Json(LoginResponse {
        user: UserData {
            id: response.user.id.to_string(),
            username: response.user.username,
            email: Some(response.user.email),
            avatar_url: response.user.avatar,
        },
        token: response.token,
    }))
}

/// Handle email verification
pub async fn verify_email(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<VerifyEmailRequest>,
) -> Result<Json<SuccessResponse>, AuthError> {
    debug!("Email verification for: {}", request.email);
    
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "verify_email".to_string())
        .with_metadata("email".to_string(), request.email.clone());
    
    let command = VerifyEmailCommand::new(request.email, request.verification_token);
    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Email verification failed: {}", e);
            AuthError::verification_failed(&e)
        })?;
    
    Ok(Json(SuccessResponse {
        message: response.message,
    }))
}

/// Resend verification email
pub async fn resend_verification_email(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ResendVerificationEmailRequest>,
) -> Json<SuccessResponse> {
    debug!("Resend verification email for: {}", request.email);
    
    let command = ResendVerificationEmailCommand::new(request.email.clone());
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "resend_verification_email".to_string())
        .with_metadata("email".to_string(), request.email.clone());

    // Execute command but handle all errors gracefully to prevent user enumeration
    match state.command_service.execute(command, context).await {
        Ok(response) => Json(SuccessResponse { message: response.message }),
        Err(e) => {
            // Log the actual error for debugging but don't reveal it to the client
            debug!("Resend verification failed: {}", e);
            
            // Always return generic success message to prevent user enumeration attacks
            // This prevents attackers from discovering which emails are registered
            Json(SuccessResponse {
                message: "If your email is registered and unverified, a verification email has been sent.".to_string(),
            })
        }
    }
}

/// Provider token response for internal endpoints
#[derive(Debug, Serialize)]
pub struct InternalProviderTokenResponse {
    /// Access token from the provider
    pub access_token: String,
    /// Token expiration in seconds (optional)
    pub expires_in: Option<u64>,
}

/// Handle internal provider token request
pub async fn internal_provider_token(
    State(state): State<AppState>,
    Valid(Path(provider_path)): Valid<Path<ProviderPath>>,
    auth_user: AuthUser,
) -> Result<Json<InternalProviderTokenResponse>, AuthError> {
    debug!("Internal provider token request for provider: {} and user: {}", provider_path.provider_name, auth_user.user_id);
    
    // Parse the provider
    let provider = match provider_path.provider_name.to_lowercase().as_str() {
        "github" => Provider::GitHub,
        "gitlab" => Provider::GitLab,
        _ => return Err(AuthError::oauth_invalid_provider("internal_token")),
    };
    
    let command = GetProviderTokenCommand::new(auth_user.user_id, provider);
    
    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "internal_provider_token".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());
    
    let result = state.command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Failed to get provider token: {}", e);
            AuthError::provider_token_failed(&e, provider.as_str())
        })?;
    
    Ok(Json(InternalProviderTokenResponse {
        access_token: result.access_token,
        expires_in: result.expires_in,
    }))
}

/// Handle JWKS endpoint - returns public keys for JWT verification
/// This endpoint is used by reverse proxies and services like Istio to validate JWT tokens
pub async fn jwks(
    State(state): State<AppState>,
) -> Result<Json<domain::entity::token::JwkSet>, AuthError> {
    debug!("JWKS endpoint requested");
    
    let jwks = state.token_usecase.get_jwks();
    
    debug!("Returning JWKS with {} keys", jwks.keys.len());
    Ok(Json(jwks))
} 