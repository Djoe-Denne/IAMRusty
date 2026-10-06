use crate::oauth_browser::{validate_authorization_pkce, CallbackIntent, OAuthRouteContext};
use crate::oauth_state::OAuthOperation;
use crate::platform_user::PlatformUser;
use crate::{error::AuthError, idp_registry, oauth_state::OAuthState};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Redirect,
    Extension, Json,
};
use axum_valid::Valid;
use iam_application::command::{
    oauth_login::{GenerateOAuthStartUrlCommand, OAuthLoginCommand},
    password_login::PasswordLoginCommand,
    provider::{
        GenerateLinkProviderStartUrlCommand, GenerateRelinkProviderStartUrlCommand,
        GetProviderTokenCommand, LinkProviderCommand, RelinkProviderCommand,
        RevokeProviderTokenCommand,
    },
    registration::{CheckUsernameCommand, CompleteRegistrationCommand},
    resend_verification_email::ResendVerificationEmailCommand,
    signup::SignupCommand,
    user::GetUserCommand,
    verify_email::VerifyEmailCommand,
    CommandContext,
};
use iam_configuration::{IdpConfig, IdpRedirectFlow};
use iam_domain::entity::oauth_transaction::ConsumedOAuthTransaction;
use iam_domain::entity::provider::Provider;
use rustycog::http::AppState;
use rustycog::http::{AuthUser, ValidatedJson};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};
use url;
use uuid::Uuid;
use validator::Validate;

/// OAuth callback query parameters
#[derive(Deserialize, Validate)]
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

impl std::fmt::Debug for OAuthCallbackQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OAuthCallbackQuery([redacted])")
    }
}

/// OAuth provider path parameter
#[derive(Debug, Deserialize)]
pub struct ProviderPath {
    /// Provider name (github, gitlab, etc.)
    pub provider_name: String,
}

/// User data for responses
#[derive(Debug, Serialize)]
pub struct UserData {
    /// User ID
    pub id: String,
    /// Username (null if registration incomplete)
    pub username: Option<String>,
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
#[derive(Serialize)]
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
    RegistrationRequired(OAuthRegistrationRequiredResponse),
}

/// OAuth registration required response
#[derive(Serialize)]
pub struct OAuthRegistrationRequiredResponse {
    /// Operation type
    pub operation: String,
    /// Registration token
    pub registration_token: String,
    /// Provider information
    pub provider_info: ProviderInfo,
    /// Whether username is required
    pub requires_username: bool,
}

/// Provider information from OAuth
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    /// Email from provider
    pub email: String,
    /// Avatar URL from provider (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Suggested username based on provider data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_username: Option<String>,
}

/// Email/password signup request (username now chosen separately)
#[derive(Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(custom(
        function = "crate::validation::validate_email_format",
        message = "Invalid email format"
    ))]
    pub email: String,
    #[validate(custom(
        function = "crate::validation::validate_strong_password",
        message = "Password must be at least 8 characters and contain both letters and numbers"
    ))]
    pub password: String,
}

/// Signup response variants
#[derive(Serialize)]
#[serde(untagged)]
pub enum SignupResponse {
    /// Existing user - password auth added
    ExistingUser {
        user: UserData,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
        message: String,
    },
    /// New user created - username required
    NewUser {
        user: UserData,
        registration_token: String,
        requires_username: bool,
        message: String,
    },
}

/// Email/password login request
#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(custom(
        function = "crate::validation::validate_email_format",
        message = "Invalid email format"
    ))]
    pub email: String,
    #[validate(custom(
        function = "crate::validation::validate_non_empty_string",
        message = "Password is required"
    ))]
    pub password: String,
}

/// Email verification request (query parameters)
#[derive(Debug, Deserialize, Validate)]
pub struct VerifyEmailQuery {
    #[validate(custom(
        function = "crate::validation::validate_email_format",
        message = "Invalid email format"
    ))]
    pub email: String,
    #[validate(custom(
        function = "crate::validation::validate_verification_token",
        message = "Invalid verification token format"
    ))]
    pub token: String,
}

/// Resend verification email request
#[derive(Debug, Deserialize, Validate)]
pub struct ResendVerificationEmailRequest {
    #[validate(custom(
        function = "crate::validation::validate_email_format",
        message = "Invalid email format"
    ))]
    pub email: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// Login response variants
#[derive(Serialize)]
#[serde(untagged)]
pub enum LoginResponse {
    /// Successful login
    Success {
        user: UserData,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
    },
    /// Registration incomplete - needs username
    RegistrationIncomplete {
        registration_token: String,
        message: String,
    },
}

fn parse_provider_slug(provider_name: &str, operation: &str) -> Result<Provider, AuthError> {
    Provider::parse_slug(provider_name).map_err(|_| AuthError::oauth_invalid_provider(operation))
}

fn require_registered_provider(
    idp: &IdpConfig,
    provider: &Provider,
    operation: &str,
) -> Result<(), AuthError> {
    if idp.has_connector(provider.as_str()) {
        Ok(())
    } else {
        Err(AuthError::oauth_connector_not_configured(operation))
    }
}

fn resolve_provider_redirect(
    idp: &IdpConfig,
    provider_name: &str,
    flow: IdpRedirectFlow,
    operation: &str,
) -> Result<(Provider, String), AuthError> {
    let provider = parse_provider_slug(provider_name, operation)?;
    let redirect_uri = idp_registry::redirect_uri_for(idp, provider.as_str(), flow)
        .ok_or_else(|| AuthError::oauth_connector_not_configured(operation))?;
    Ok((provider, redirect_uri))
}

/// Handle OAuth login start - redirects to provider for login (unauthenticated users)
///
/// # Errors
///
/// Returns [`AuthError`] when the provider is unknown, OAuth state encoding fails,
/// the authorization URL cannot be generated, or the generated URL is invalid.
pub async fn oauth_login_start(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Extension(oauth): Extension<Arc<OAuthRouteContext>>,
    Path(provider_path): Path<ProviderPath>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Redirect), AuthError> {
    debug!(
        "OAuth login start for provider: {}",
        provider_path.provider_name
    );

    let (provider, redirect_uri) = resolve_provider_redirect(
        &idp,
        &provider_path.provider_name,
        IdpRedirectFlow::Callback,
        "login_start",
    )?;

    // Create login state
    debug!("Creating login state");
    let oauth_state = OAuthState::new_login(provider.as_str());

    // Encode the state
    let encoded_state = oauth_state
        .encode()
        .map_err(|_e| AuthError::oauth_state_encoding_failed("login_start"))?;
    let nonce = oauth
        .browser
        .start_nonce(&headers)
        .map_err(|_| AuthError::oauth_invalid_state("login_start"))?;
    let pkce_required = idp
        .connector(provider.as_str())
        .is_some_and(|cfg| cfg.pkce_supported);
    let begun = oauth
        .begin(
            &oauth_state,
            &encoded_state,
            &nonce,
            provider.clone(),
            redirect_uri.clone(),
            pkce_required,
        )
        .await?;

    // Generate provider authorization URL using the command service
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "login_start".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());

    let command = GenerateOAuthStartUrlCommand::new(
        provider,
        redirect_uri,
        encoded_state.clone(),
        begun.clone(),
    );
    let base_auth_url = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|_e| AuthError::oauth_url_generation_failed("login_start"))?;

    // Parse the URL and replace the state parameter with our own
    let mut url = url::Url::parse(&base_auth_url)
        .map_err(|_e| AuthError::oauth_invalid_url("login_start"))?;
    oauth.browser.validate_authorization_url(&url)?;
    validate_authorization_pkce(&url, &begun)?;

    // Remove any existing state parameter and add our own
    let mut new_query_pairs = Vec::new();
    for (key, value) in url.query_pairs() {
        if key != "state" {
            new_query_pairs.push((key.to_string(), value.to_string()));
        }
    }
    new_query_pairs.push(("state".to_string(), encoded_state));

    // Clear existing query and set new query pairs
    url.set_query(None);
    {
        let mut query_pairs = url.query_pairs_mut();
        for (key, value) in new_query_pairs {
            query_pairs.append_pair(&key, &value);
        }
        query_pairs.finish();
    }

    debug!("Redirecting to provider for login");
    let cookies = oauth
        .browser
        .cookie_headers(&nonce)
        .map_err(|_| AuthError::oauth_invalid_state("login_start"))?;
    Ok((cookies, Redirect::to(url.as_str())))
}

/// Handle OAuth link start - redirects to provider for linking (authenticated users)
///
/// # Errors
///
/// Returns [`AuthError`] when the provider is unknown, OAuth state encoding fails,
/// the authorization URL cannot be generated, or the generated URL is invalid.
pub async fn oauth_link_start(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Extension(oauth): Extension<Arc<OAuthRouteContext>>,
    Path(provider_path): Path<ProviderPath>,
    auth_user: PlatformUser,
    headers: HeaderMap,
) -> Result<(HeaderMap, Redirect), AuthError> {
    debug!(
        "OAuth link start for provider: {} and user: {}",
        provider_path.provider_name, auth_user.user_id
    );

    let (provider, redirect_uri) = resolve_provider_redirect(
        &idp,
        &provider_path.provider_name,
        IdpRedirectFlow::Callback,
        "link_start",
    )?;

    // check if user exists
    let user_context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "get_user".to_string());
    let _user = state
        .command_service
        .execute(GetUserCommand::new(auth_user.user_id), user_context)
        .await
        .map_err(|_e| {
            tracing::error!("User not found for id: {}", auth_user.user_id);
            AuthError::oauth_invalid_token("link_start")
        })?;

    // Create link state for authenticated user
    debug!("Creating link state for user: {}", auth_user.user_id);
    let oauth_state = OAuthState::new_link(auth_user.user_id, provider.as_str());

    // Encode the state
    let encoded_state = oauth_state
        .encode()
        .map_err(|_e| AuthError::oauth_state_encoding_failed("link_start"))?;
    let nonce = oauth
        .browser
        .start_nonce(&headers)
        .map_err(|_| AuthError::oauth_invalid_state("link_start"))?;
    let pkce_required = idp
        .connector(provider.as_str())
        .is_some_and(|cfg| cfg.pkce_supported);
    let begun = oauth
        .begin(
            &oauth_state,
            &encoded_state,
            &nonce,
            provider.clone(),
            redirect_uri.clone(),
            pkce_required,
        )
        .await?;

    // Generate provider authorization URL using the command service
    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "link_start".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());

    let command = GenerateLinkProviderStartUrlCommand::new(
        provider,
        redirect_uri,
        encoded_state.clone(),
        begun.clone(),
    );
    let base_auth_url = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|_e| AuthError::oauth_url_generation_failed("link_start"))?;

    // Parse the URL and replace the state parameter with our own
    let mut url =
        url::Url::parse(&base_auth_url).map_err(|_e| AuthError::oauth_invalid_url("link_start"))?;
    oauth.browser.validate_authorization_url(&url)?;
    validate_authorization_pkce(&url, &begun)?;

    // Remove any existing state parameter and add our own
    let mut new_query_pairs = Vec::new();
    for (key, value) in url.query_pairs() {
        if key != "state" {
            new_query_pairs.push((key.to_string(), value.to_string()));
        }
    }
    new_query_pairs.push(("state".to_string(), encoded_state));

    // Clear existing query and set new query pairs
    url.set_query(None);
    {
        let mut query_pairs = url.query_pairs_mut();
        for (key, value) in new_query_pairs {
            query_pairs.append_pair(&key, &value);
        }
        query_pairs.finish();
    }

    debug!("Redirecting to provider for linking");
    let cookies = oauth
        .browser
        .cookie_headers(&nonce)
        .map_err(|_| AuthError::oauth_invalid_state("link_start"))?;
    Ok((cookies, Redirect::to(url.as_str())))
}

/// Handle OAuth callback - processes both login and link operations
///
/// # Errors
///
/// Returns [`AuthError`] when the provider reports an error, the authorization code
/// or state is missing or invalid, the provider is unknown, or login/link command
/// execution fails.
pub async fn oauth_callback(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Extension(oauth): Extension<Arc<OAuthRouteContext>>,
    Path(provider_path): Path<ProviderPath>,
    headers: HeaderMap,
    Valid(Query(query)): Valid<Query<OAuthCallbackQuery>>,
) -> Result<(StatusCode, Json<OAuthResponse>), AuthError> {
    debug!(
        "OAuth callback for provider: {}",
        provider_path.provider_name
    );

    let (provider, redirect_uri) = resolve_provider_redirect(
        &idp,
        &provider_path.provider_name,
        IdpRedirectFlow::Callback,
        "callback",
    )?;

    // Decode the state to determine operation type
    let state_param = query
        .state
        .as_deref()
        .ok_or_else(|| AuthError::oauth_missing_state("callback"))?;
    let consumed = oauth
        .consume_callback(
            state_param,
            &headers,
            provider,
            redirect_uri,
            CallbackIntent::LoginOrLink,
        )
        .await?;
    // An error callback terminates the same browser-bound transaction, without exchange.
    if let Some(error) = query.error {
        return Err(AuthError::oauth_provider_error(
            "callback",
            error,
            query.error_description.unwrap_or_default(),
        ));
    }
    let code = query
        .code
        .filter(|code| !code.trim().is_empty())
        .ok_or_else(|| AuthError::oauth_missing_code("callback"))?;
    let provider = consumed.provider().clone();
    let redirect_uri = consumed.redirect_uri().to_owned();

    if matches!(consumed.operation(), OAuthOperation::Login) {
        // Handle login operation
        let (status_code, json_response) =
            handle_login_callback(state, provider, code, redirect_uri, consumed).await?;
        Ok((status_code, json_response))
    } else if matches!(consumed.operation(), OAuthOperation::Link { .. }) {
        let user_id = consumed
            .target_user_id()
            .ok_or_else(|| AuthError::oauth_invalid_state("callback"))?;
        // Handle link operation
        debug!("handle_link_callback user={user_id}");
        let json_response =
            handle_link_callback(state, provider, code, redirect_uri, user_id, consumed).await?;
        Ok((StatusCode::OK, json_response))
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
    consumed: ConsumedOAuthTransaction,
) -> Result<(StatusCode, Json<OAuthResponse>), AuthError> {
    debug!("Handling login callback");

    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "login_callback".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());

    let command = OAuthLoginCommand::new(provider, code, redirect_uri, consumed);
    // The committed consume proof is moved once; transport failures never auto-retry.
    let response = state
        .command_service
        .execute_once(command, context)
        .await
        .map_err(|e| {
            error!("Failed to login");
            AuthError::oauth_login_failed("login", &e)
        })?;

    // Handle both login and registration scenarios
    match response {
        iam_application::usecase::oauth::OAuthResponse::Login(login_response) => {
            // Existing complete user - return 200 with login tokens
            Ok((
                StatusCode::OK,
                Json(OAuthResponse::Login(OAuthLoginResponse {
                    operation: "login".to_string(),
                    user: UserData {
                        id: login_response.user.id.to_string(),
                        username: login_response.user.username.clone(),
                        email: Some(login_response.email),
                        avatar_url: login_response.user.avatar_url,
                    },
                    access_token: login_response.access_token,
                    expires_in: login_response.expires_in, // Now using actual expiration from domain service
                    refresh_token: login_response.refresh_token, // Now using actual refresh token from domain service
                })),
            ))
        }
        iam_application::usecase::oauth::OAuthResponse::Registration(reg_response) => {
            // New user needs to complete registration - return 202 with registration token
            Ok((
                StatusCode::ACCEPTED,
                Json(OAuthResponse::RegistrationRequired(
                    OAuthRegistrationRequiredResponse {
                        operation: "registration_required".to_string(),
                        registration_token: reg_response.registration_token,
                        provider_info: ProviderInfo {
                            email: reg_response.provider_info.email,
                            avatar: reg_response.provider_info.avatar_url,
                            suggested_username: reg_response.provider_info.username,
                        },
                        requires_username: true,
                    },
                )),
            ))
        }
    }
}

/// Handle link provider callback
async fn handle_link_callback(
    state: AppState,
    provider: Provider,
    code: String,
    redirect_uri: String,
    user_id: Uuid,
    consumed: ConsumedOAuthTransaction,
) -> Result<Json<OAuthResponse>, AuthError> {
    debug!("Handling link callback for user: {}", user_id);

    let slug = provider.as_str().to_string();
    let context = CommandContext::new()
        .with_user_id(user_id)
        .with_metadata("operation".to_string(), "link_callback".to_string())
        .with_metadata("provider".to_string(), slug.clone());

    let command = LinkProviderCommand::new(user_id, provider, code, redirect_uri, consumed);
    // A callback is non-replayable after consume, even when the provider fails.
    let response = state
        .command_service
        .execute_once(command, context)
        .await
        .map_err(|e| {
            error!("Failed to link provider");
            AuthError::oauth_link_failed("link", &e, &slug)
        })?;

    // Convert UserEmail entities to EmailData
    let emails: Vec<EmailData> = response
        .emails
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
        message: format!("{slug} successfully linked"),
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
///
/// # Errors
///
/// Returns [`AuthError`] when the signup command fails (duplicate email, persistence,
/// hashing, or token generation).
#[axum::debug_handler]
pub async fn signup(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<SignupRequest>,
) -> Result<(StatusCode, Json<SignupResponse>), AuthError> {
    debug!("Email/password signup");

    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "signup".to_string())
        .with_metadata("email".to_string(), request.email.clone());

    let command = SignupCommand::new(request.email, request.password);
    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Signup failed");
            AuthError::signup_failed(&e)
        })?;

    match response {
        iam_domain::service::auth_service::SignupResponse::ExistingUser {
            user,
            access_token,
            expires_in,
            refresh_token,
            message,
        } => Ok((
            StatusCode::OK,
            Json(SignupResponse::ExistingUser {
                user: UserData {
                    id: user.id.to_string(),
                    username: user.username,
                    email: Some(user.email),
                    avatar_url: user.avatar,
                },
                access_token,
                expires_in,
                refresh_token,
                message,
            }),
        )),
        iam_domain::service::auth_service::SignupResponse::RegistrationRequired {
            user,
            registration_token,
            requires_username,
            message,
        } => {
            Ok((
                StatusCode::ACCEPTED,
                Json(SignupResponse::NewUser {
                    user: UserData {
                        id: user.id.to_string(),
                        username: None,
                        email: Some(user.email),
                        avatar_url: None, // Incomplete user doesn't have avatar yet
                    },
                    registration_token,
                    requires_username,
                    message,
                }),
            ))
        }
    }
}

/// Handle email/password login
///
/// # Errors
///
/// Returns [`AuthError`] when credentials are rejected, the account is not fully
/// registered ([`AuthError::RegistrationIncomplete`]), or the login command fails.
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    debug!("Email/password login");

    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "login".to_string())
        .with_metadata("email".to_string(), request.email.clone());

    let command = PasswordLoginCommand::new(request.email, request.password);
    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Login failed");
            AuthError::login_failed(&e)
        })?;

    match response {
        iam_domain::service::auth_service::LoginResponse::Success {
            user,
            access_token,
            expires_in,
            refresh_token,
        } => Ok(Json(LoginResponse::Success {
            user: UserData {
                id: user.id.to_string(),
                username: user.username,
                email: Some(user.email),
                avatar_url: user.avatar,
            },
            access_token,
            expires_in,
            refresh_token,
        })),
        iam_domain::service::auth_service::LoginResponse::RegistrationIncomplete {
            registration_token,
            message,
        } => Err(AuthError::RegistrationIncomplete {
            registration_token,
            message,
        }),
    }
}

/// Handle email verification
///
/// # Errors
///
/// Returns [`AuthError`] when the verification token is invalid or the verify-email
/// command fails.
pub async fn verify_email(
    State(state): State<AppState>,
    Valid(Query(request)): Valid<Query<VerifyEmailQuery>>,
) -> Result<Json<SuccessResponse>, AuthError> {
    debug!("Email verification");

    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "verify_email".to_string())
        .with_metadata("email".to_string(), request.email.clone());

    let command = VerifyEmailCommand::new(request.email, request.token);
    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Email verification failed");
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
    debug!("Resend verification email");

    let command = ResendVerificationEmailCommand::new(request.email.clone());
    let context = CommandContext::new()
        .with_metadata(
            "operation".to_string(),
            "resend_verification_email".to_string(),
        )
        .with_metadata("email".to_string(), request.email.clone());

    // Execute command but handle all errors gracefully to prevent user enumeration
    match state.command_service.execute(command, context).await {
        Ok(response) => Json(SuccessResponse {
            message: response.message,
        }),
        Err(_e) => {
            // Emit only the operation, never raw upstream diagnostic text.
            debug!("Resend verification failed");

            // Always return generic success message to prevent user enumeration attacks
            // This prevents attackers from discovering which emails are registered
            Json(SuccessResponse {
                message: "If your email is registered and unverified, a verification email has been sent.".to_string(),
            })
        }
    }
}

/// Provider token response for internal endpoints
#[derive(Serialize)]
pub struct InternalProviderTokenResponse {
    /// Access token from the provider
    pub access_token: String,
    /// Token expiration in seconds (optional)
    pub expires_in: Option<u64>,
}

/// Handle internal provider token request
///
/// # Errors
///
/// Returns [`AuthError`] when the provider is unknown or the stored provider token
/// cannot be retrieved.
pub async fn internal_provider_token(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Path(provider_path): Path<ProviderPath>,
    headers: HeaderMap,
    auth_user: AuthUser,
) -> Result<Json<InternalProviderTokenResponse>, AuthError> {
    crate::rate_limit::require_internal_service_token(&headers).map_err(|_| AuthError::OAuth {
        operation: "internal_token".to_string(),
        error_code: "forbidden".to_string(),
        message: "internal service token required".to_string(),
        status: StatusCode::FORBIDDEN,
    })?;
    debug!(
        "Internal provider token request for provider: {} and user: {}",
        provider_path.provider_name, auth_user.user_id
    );

    let provider = parse_provider_slug(&provider_path.provider_name, "internal_token")?;
    require_registered_provider(&idp, &provider, "internal_token")?;
    let slug = provider.as_str().to_string();

    let command = GetProviderTokenCommand::new(auth_user.user_id, provider);

    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata(
            "operation".to_string(),
            "internal_provider_token".to_string(),
        )
        .with_metadata("provider".to_string(), slug.clone());

    let result = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Failed to get provider token");
            AuthError::provider_token_failed(&e, &slug)
        })?;

    Ok(Json(InternalProviderTokenResponse {
        access_token: result.access_token,
        expires_in: result.expires_in,
    }))
}

/// Handle JWKS endpoint - returns public keys for JWT verification
///
/// This endpoint is used by reverse proxies and services like Istio to validate JWT tokens.
///
/// # Errors
///
/// Returns [`AuthError`] when the JWKS command fails to load the public keys.
pub async fn jwks(
    State(state): State<AppState>,
) -> Result<Json<iam_domain::entity::token::JwkSet>, AuthError> {
    debug!("JWKS endpoint requested");

    let context =
        CommandContext::new().with_metadata("operation".to_string(), "get_jwks".to_string());

    let command = iam_application::command::token::GetJwksCommand::new();
    let jwks = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|_e| {
            error!("Failed to get JWKS");
            AuthError::oauth_url_generation_failed("get_jwks")
        })?;

    debug!("Returning JWKS with {} keys", jwks.keys.len());
    Ok(Json(jwks))
}

/// Complete registration request
#[derive(Deserialize, Validate)]
pub struct CompleteRegistrationRequest {
    /// RSA-signed JWT registration token
    #[validate(length(min = 1, message = "Registration token is required"))]
    pub registration_token: String,
    /// Chosen username
    #[validate(length(
        min = 3,
        max = 50,
        message = "Username must be between 3 and 50 characters"
    ))]
    #[validate(custom(
        function = "crate::validation::validate_username",
        message = "Username must be 3-50 characters and contain only letters, numbers, underscores, and hyphens"
    ))]
    pub username: String,
}

/// Complete registration response
#[derive(Serialize)]
pub struct CompleteRegistrationResponse {
    /// User profile
    pub user: UserData,
    /// Access token
    pub access_token: String,
    /// Token expiration in seconds
    pub expires_in: u64,
    /// Refresh token
    pub refresh_token: String,
}

/// Check username request (query parameter)
#[derive(Debug, Deserialize, Validate)]
pub struct CheckUsernameQuery {
    /// Username to check
    #[validate(length(
        min = 3,
        max = 50,
        message = "Username must be between 3 and 50 characters"
    ))]
    #[validate(custom(
        function = "crate::validation::validate_username",
        message = "Username can only contain letters, numbers, underscores, and hyphens"
    ))]
    pub username: String,
}

/// Username availability response
#[derive(Debug, Serialize)]
pub struct CheckUsernameResponse {
    /// Whether the username is available
    pub available: bool,
    /// Suggested alternatives if username is taken (always present, empty array if available)
    pub suggestions: Vec<String>,
}

/// Handle complete registration endpoint
///
/// # Errors
///
/// Returns [`AuthError`] when the registration token is invalid or completing
/// registration fails.
pub async fn complete_registration(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<CompleteRegistrationRequest>,
) -> Result<Json<CompleteRegistrationResponse>, AuthError> {
    debug!("Complete registration for username: {}", request.username);

    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "complete_registration".to_string())
        .with_metadata("username".to_string(), request.username.clone());

    let command = CompleteRegistrationCommand::new(request.registration_token, request.username);

    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Complete registration failed");
            AuthError::registration_failed(&e)
        })?;

    Ok(Json(CompleteRegistrationResponse {
        user: UserData {
            id: response.user.id,
            username: Some(response.user.username),
            email: Some(response.user.email),
            avatar_url: response.user.avatar,
        },
        access_token: response.access_token,
        expires_in: response.expires_in,
        refresh_token: response.refresh_token,
    }))
}

/// Handle username availability check endpoint
///
/// # Errors
///
/// Returns [`AuthError`] when the username check command fails.
pub async fn check_username(
    State(state): State<AppState>,
    Valid(Query(query)): Valid<Query<CheckUsernameQuery>>,
) -> Result<Json<CheckUsernameResponse>, AuthError> {
    debug!("Check username availability for: {}", query.username);

    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "check_username".to_string())
        .with_metadata("username".to_string(), query.username.clone());

    let command = CheckUsernameCommand::new(query.username);

    let response = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Username check failed");
            AuthError::username_check_failed(&e)
        })?;

    Ok(Json(CheckUsernameResponse {
        available: response.available,
        suggestions: response.suggestions.unwrap_or_else(Vec::new),
    }))
}

/// Revoke provider token response
#[derive(Debug, Serialize)]
pub struct RevokeProviderTokenResponse {
    /// Success message
    pub message: String,
}

/// Handle revoke provider token request
///
/// # Errors
///
/// Returns [`AuthError`] when the provider is unknown or revoking the provider token
/// fails, or when the internal service token is missing or invalid.
pub async fn revoke_provider_token(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Path(provider_path): Path<ProviderPath>,
    headers: HeaderMap,
    auth_user: AuthUser,
) -> Result<Json<RevokeProviderTokenResponse>, AuthError> {
    crate::rate_limit::require_internal_service_token(&headers).map_err(|_| AuthError::OAuth {
        operation: "revoke_provider_token".to_string(),
        error_code: "forbidden".to_string(),
        message: "internal service token required".to_string(),
        status: StatusCode::FORBIDDEN,
    })?;
    debug!(
        "Revoke provider token request for provider: {} and user: {}",
        provider_path.provider_name, auth_user.user_id
    );

    let provider = parse_provider_slug(&provider_path.provider_name, "revoke_provider_token")?;
    require_registered_provider(&idp, &provider, "revoke_provider_token")?;
    let slug = provider.as_str().to_string();

    let command = RevokeProviderTokenCommand::new(auth_user.user_id, provider);

    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "revoke_provider_token".to_string())
        .with_metadata("provider".to_string(), slug.clone());

    state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Failed to revoke provider token");
            AuthError::provider_token_failed(&e, &slug)
        })?;

    Ok(Json(RevokeProviderTokenResponse {
        message: format!("Provider {slug} token revoked successfully"),
    }))
}

/// OAuth start response for generating authorization URLs
#[derive(Serialize)]
pub struct OAuthStartResponse {
    /// Authorization URL to redirect user to
    pub auth_url: String,
}

impl std::fmt::Debug for OAuthStartResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OAuthStartResponse([redacted])")
    }
}

macro_rules! redacted_auth_debug {
    ($($name:ident),+ $(,)?) => { $(
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    )+ };
}

redacted_auth_debug!(
    OAuthLoginResponse,
    OAuthRegistrationRequiredResponse,
    SignupRequest,
    SignupResponse,
    LoginRequest,
    LoginResponse,
    InternalProviderTokenResponse,
    CompleteRegistrationRequest,
    CompleteRegistrationResponse
);

/// Relink provider callback response
#[derive(Debug, Serialize)]
pub struct RelinkProviderCallbackResponse {
    /// User data
    pub user: UserData,
    /// All user emails (including any newly added)
    pub emails: Vec<EmailData>,
    /// Whether a new email was added during relinking
    pub new_email_added: bool,
    /// The new email that was added (if any)
    pub new_email: Option<String>,
}

/// Handle relink provider callback
///
/// # Errors
///
/// Returns [`AuthError`] when the provider is unknown, the OAuth callback is invalid,
/// or the relink command fails.
pub async fn relink_provider_callback(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Extension(oauth): Extension<Arc<OAuthRouteContext>>,
    Path(provider_path): Path<ProviderPath>,
    headers: HeaderMap,
    Valid(Query(callback_request)): Valid<Query<OAuthCallbackQuery>>,
) -> Result<Json<RelinkProviderCallbackResponse>, AuthError> {
    debug!(
        "Relink provider callback for provider: {}",
        provider_path.provider_name
    );

    let (provider, redirect_uri) = resolve_provider_redirect(
        &idp,
        &provider_path.provider_name,
        IdpRedirectFlow::Relink,
        "relink_provider",
    )?;

    let state_param = callback_request
        .state
        .as_deref()
        .ok_or_else(|| AuthError::oauth_missing_state("relink_provider"))?;
    let consumed = oauth
        .consume_callback(
            state_param,
            &headers,
            provider,
            redirect_uri,
            CallbackIntent::Relink,
        )
        .await?;
    if let Some(error) = callback_request.error {
        return Err(AuthError::oauth_provider_error(
            "relink_provider",
            error,
            callback_request.error_description.unwrap_or_default(),
        ));
    }
    let code = callback_request
        .code
        .filter(|code| !code.trim().is_empty())
        .ok_or_else(|| AuthError::oauth_missing_code("relink_provider"))?;
    let user_id = consumed
        .target_user_id()
        .ok_or_else(|| AuthError::oauth_invalid_state("relink_provider"))?;
    let provider = consumed.provider().clone();
    let redirect_uri = consumed.redirect_uri().to_owned();
    let slug = provider.as_str().to_string();
    let command = RelinkProviderCommand::new(user_id, provider, code, redirect_uri, consumed);

    let context = CommandContext::new()
        .with_user_id(user_id)
        .with_metadata(
            "operation".to_string(),
            "relink_provider_callback".to_string(),
        )
        .with_metadata("provider".to_string(), slug.clone());

    // Share no consumed proof with the retrying bus; relink executes at most once.
    let result = state
        .command_service
        .execute_once(command, context)
        .await
        .map_err(|e| {
            error!("Failed to relink provider");
            AuthError::link_provider_failed(&e, &slug)
        })?;

    // Find primary email for user data
    let primary_email = result
        .emails
        .iter()
        .find(|e| e.is_primary)
        .map(|e| e.email.clone());

    Ok(Json(RelinkProviderCallbackResponse {
        user: UserData {
            id: result.user.id.to_string(),
            username: result.user.username,
            email: primary_email,
            avatar_url: result.user.avatar_url,
        },
        emails: result
            .emails
            .into_iter()
            .map(|e| EmailData {
                id: e.id.to_string(),
                email: e.email,
                is_primary: e.is_primary,
                is_verified: e.is_verified,
            })
            .collect(),
        new_email_added: result.new_email_added,
        new_email: result.new_email,
    }))
}

/// Handle generate relink provider start URL
///
/// # Errors
///
/// Returns [`AuthError`] when the provider is unknown or the authorization URL cannot
/// be generated.
pub async fn generate_relink_provider_start_url(
    State(state): State<AppState>,
    Extension(idp): Extension<Arc<IdpConfig>>,
    Extension(oauth): Extension<Arc<OAuthRouteContext>>,
    Path(provider_path): Path<ProviderPath>,
    auth_user: PlatformUser,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<OAuthStartResponse>), AuthError> {
    debug!(
        "Generate relink provider start URL for provider: {}",
        provider_path.provider_name
    );

    let (provider, redirect_uri) = resolve_provider_redirect(
        &idp,
        &provider_path.provider_name,
        IdpRedirectFlow::Relink,
        "generate_relink_provider_start_url",
    )?;

    let slug = provider.as_str().to_string();
    let oauth_state = OAuthState::new_relink(auth_user.user_id, provider.as_str());
    let encoded_state = oauth_state
        .encode()
        .map_err(|_| AuthError::oauth_state_encoding_failed("relink_start"))?;
    let nonce = oauth
        .browser
        .start_nonce(&headers)
        .map_err(|_| AuthError::oauth_invalid_state("relink_start"))?;
    let pkce_required = idp
        .connector(provider.as_str())
        .is_some_and(|cfg| cfg.pkce_supported);
    let begun = oauth
        .begin(
            &oauth_state,
            &encoded_state,
            &nonce,
            provider.clone(),
            redirect_uri.clone(),
            pkce_required,
        )
        .await?;
    let command = GenerateRelinkProviderStartUrlCommand::new(
        provider,
        redirect_uri,
        encoded_state.clone(),
        begun.clone(),
    );

    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata(
            "operation".to_string(),
            "generate_relink_provider_start_url".to_string(),
        )
        .with_metadata("provider".to_string(), slug.clone());

    let auth_url = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| {
            error!("Failed to generate relink provider start URL");
            AuthError::oauth_start_failed(&e, &slug)
        })?;

    let mut url =
        url::Url::parse(&auth_url).map_err(|_| AuthError::oauth_invalid_url("relink_start"))?;
    oauth.browser.validate_authorization_url(&url)?;
    validate_authorization_pkce(&url, &begun)?;
    let pairs: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key != "state")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        query.extend_pairs(pairs);
        query.append_pair("state", &encoded_state);
        query.finish();
    }
    let cookies = oauth
        .browser
        .cookie_headers(&nonce)
        .map_err(|_| AuthError::oauth_invalid_state("relink_start"))?;
    Ok((
        cookies,
        Json(OAuthStartResponse {
            auth_url: url.to_string(),
        }),
    ))
}

#[cfg(test)]
mod credential_debug_tests {
    use super::*;

    #[test]
    fn oauth_browser_credentials_remain_wire_data_not_diagnostics() {
        let query = OAuthCallbackQuery {
            code: Some("SENTINEL-CODE".into()),
            state: Some("SENTINEL-STATE".into()),
            error: None,
            error_description: None,
        };
        let start = OAuthStartResponse {
            auth_url: "https://vendor/authorize?state=SENTINEL-STATE".into(),
        };
        let login = OAuthLoginResponse {
            operation: "login".into(),
            user: UserData {
                id: uuid::Uuid::nil().to_string(),
                username: None,
                email: None,
                avatar_url: None,
            },
            access_token: "SENTINEL-ACCESS".into(),
            refresh_token: "SENTINEL-REFRESH".into(),
            expires_in: 900,
        };
        assert!(!format!("{query:?} {start:?} {login:?}").contains("SENTINEL"));
        // Redaction must not break the public JSON contract.
        let wire = serde_json::to_value(&login).unwrap();
        assert_eq!(wire["refresh_token"], "SENTINEL-REFRESH");
        assert_eq!(
            serde_json::to_value(start).unwrap()["auth_url"],
            "https://vendor/authorize?state=SENTINEL-STATE"
        );
    }
}
