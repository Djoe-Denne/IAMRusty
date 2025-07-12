use iam_application::command::CommandError;
use iam_application::usecase::{token::TokenError, user::UserError};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use iam_domain::error::DomainError;
use rustycog_http::error::{UniformErrorResponse, ErrorDetails};
use thiserror::Error;

/// API errors
#[derive(Debug, Error)]
pub enum ApiError {
    /// Domain error
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Command error
    #[error(transparent)]
    Command(#[from] CommandError),

    /// User use case error
    #[error(transparent)]
    User(#[from] UserError),

    /// Token use case error
    #[error(transparent)]
    Token(#[from] TokenError),

    /// Authentication required
    #[error("Authentication required")]
    AuthenticationRequired,

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

/// Auth error type for OAuth endpoints
#[derive(Debug, Error)]
pub enum AuthError {
    /// OAuth error with specific response format
    #[error("{message}")]
    OAuth {
        operation: String,
        error_code: String,
        message: String,
        status: StatusCode,
    },

    /// Invalid provider
    #[error("Invalid provider")]
    InvalidProvider,

    /// Invalid authorization header
    #[error("Invalid authorization header: {0}")]
    InvalidAuthorizationHeader(String),

    /// Invalid token
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// State encoding failed
    #[error("State encoding failed: {0}")]
    StateEncodingFailed(String),

    /// URL generation failed
    #[error("URL generation failed: {0}")]
    UrlGenerationFailed(String),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// OAuth error from provider
    #[error("OAuth error: {0} - {1}")]
    OAuthError(String, String),

    /// Missing code parameter
    #[error("Missing code parameter")]
    MissingCode,

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Missing state parameter
    #[error("Missing state parameter")]
    MissingState,

    /// Invalid state operation
    #[error("Invalid state operation")]
    InvalidStateOperation,

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Login failed
    #[error("Login failed")]
    LoginFailed,

    /// Provider already linked to same user
    #[error("Provider already linked to same user: {0}")]
    ProviderAlreadyLinkedToSameUser(String),

    /// Provider already linked to another user
    #[error("Provider already linked: {0}")]
    ProviderAlreadyLinked(String),

    /// User not found
    #[error("Unauthorized")]
    UserNotFound(String),

    /// Link failed
    #[error("Link failed")]
    LinkFailed,

    /// General API error
    #[error(transparent)]
    Api(#[from] ApiError),

    /// Registration incomplete with token
    #[error("{message}")]
    RegistrationIncomplete {
        registration_token: String,
        message: String,
    },
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::OAuth {
                operation: _operation,
                error_code,
                message,
                status,
            } => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code,
                        message,
                        status: status.as_u16(),
                    },
                });
                (status, body).into_response()
            }
            AuthError::InvalidProvider => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "invalid_provider".to_string(),
                        message: "Invalid provider".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidAuthorizationHeader(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "invalid_authorization_header".to_string(),
                        message: "Invalid Authorization header".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidToken(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "invalid_token".to_string(),
                        message: "Invalid or expired token".to_string(),
                        status: StatusCode::UNAUTHORIZED.as_u16(),
                    },
                });
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AuthError::StateEncodingFailed(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "state_encoding_failed".to_string(),
                        message: "Failed to create OAuth state".to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    },
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::UrlGenerationFailed(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "url_generation_failed".to_string(),
                        message: "Failed to generate authorization URL".to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    },
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::InvalidUrl(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "invalid_url".to_string(),
                        message: "Invalid URL in OAuth callback".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::OAuthError(error, description) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "oauth_provider_error".to_string(),
                        message: format!("OAuth provider error: {} - {}", error, description),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::MissingCode => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "missing_code".to_string(),
                        message: "Missing authorization code".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidState(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "invalid_state".to_string(),
                        message: "Invalid OAuth state parameter".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::MissingState => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "missing_state".to_string(),
                        message: "Missing OAuth state parameter".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidStateOperation => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "invalid_state_operation".to_string(),
                        message: "Invalid OAuth state operation".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    },
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::AuthenticationFailed(_) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "authentication_failed".to_string(),
                        message: "Authentication failed".to_string(),
                        status: StatusCode::UNAUTHORIZED.as_u16(),
                    },
                });
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AuthError::ValidationFailed(msg) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "validation_failed".to_string(),
                        message: msg,
                        status: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
                    },
                });
                (StatusCode::UNPROCESSABLE_ENTITY, body).into_response()
            }
            AuthError::LoginFailed => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "login_failed".to_string(),
                        message: "Login failed".to_string(),
                        status: StatusCode::UNAUTHORIZED.as_u16(),
                    },
                });
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AuthError::ProviderAlreadyLinkedToSameUser(provider) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "provider_already_linked_same_user".to_string(),
                        message: format!("Provider {} is already linked to this user", provider),
                        status: StatusCode::CONFLICT.as_u16(),
                    },
                });
                (StatusCode::CONFLICT, body).into_response()
            }
            AuthError::ProviderAlreadyLinked(provider) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "provider_already_linked".to_string(),
                        message: format!("Provider {} is already linked to another user", provider),
                        status: StatusCode::CONFLICT.as_u16(),
                    },
                });
                (StatusCode::CONFLICT, body).into_response()
            }
            AuthError::UserNotFound(user_id) => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "user_not_found".to_string(),
                        message: format!("User not found: {}", user_id),
                        status: StatusCode::NOT_FOUND.as_u16(),
                    },
                });
                (StatusCode::NOT_FOUND, body).into_response()
            }
            AuthError::LinkFailed => {
                let body = Json(UniformErrorResponse {
                    error: ErrorDetails {
                        error_code: "link_failed".to_string(),
                        message: "Failed to link provider".to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    },
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::Api(api_error) => api_error.into_response(),
            AuthError::RegistrationIncomplete {
                registration_token,
                message,
            } => {
                let body = Json(serde_json::json!({
                    "error": "registration_incomplete",
                    "message": message,
                    "registration_token": registration_token
                }));
                (StatusCode::LOCKED, body).into_response()
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::Domain(domain_error) => {
                match domain_error {
                    DomainError::UserNotFound => (
                        StatusCode::NOT_FOUND,
                        "user_not_found".to_string(),
                        "User not found".to_string(),
                    ),
                    DomainError::ProviderNotSupported(msg) => (
                        StatusCode::BAD_REQUEST,
                        "provider_not_supported".to_string(),
                        msg,
                    ),
                    DomainError::BusinessRuleViolation(msg) => (
                        StatusCode::BAD_REQUEST,
                        "business_rule_violation".to_string(),
                        msg,
                    ),
                    DomainError::InvalidToken => (
                        StatusCode::UNAUTHORIZED,
                        "invalid_token".to_string(),
                        "Invalid token".to_string(),
                    ),
                    DomainError::TokenExpired => (
                        StatusCode::UNAUTHORIZED,
                        "token_expired".to_string(),
                        "Token expired".to_string(),
                    ),
                    DomainError::AuthorizationError(msg) => (
                        StatusCode::UNAUTHORIZED,
                        "authorization_error".to_string(),
                        msg,
                    ),
                    DomainError::OAuth2Error(msg) => {
                        (StatusCode::BAD_REQUEST, "oauth2_error".to_string(), msg)
                    }
                    DomainError::UserProfileError(msg) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "user_profile_error".to_string(),
                        msg,
                    ),
                    DomainError::NoTokenForProvider => (
                        StatusCode::NOT_FOUND,
                        "no_token_for_provider".to_string(),
                        "No token found for provider and user".to_string(),
                    ),
                    DomainError::TokenGenerationFailed(msg) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "token_generation_failed".to_string(),
                        msg,
                    ),
                    DomainError::TokenValidationFailed(msg) => (
                        StatusCode::UNAUTHORIZED,
                        "token_validation_failed".to_string(),
                        msg,
                    ),
                    DomainError::RepositoryError(msg) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "repository_error".to_string(),
                        msg,
                    ),
                    // Registration-specific errors
                    DomainError::UsernameTaken => (
                        StatusCode::CONFLICT,
                        "username_taken".to_string(),
                        "Username already taken".to_string(),
                    ),
                    DomainError::InvalidUsername => (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "invalid_username".to_string(),
                        "Invalid username format".to_string(),
                    ),
                    DomainError::RegistrationAlreadyComplete => (
                        StatusCode::BAD_REQUEST,
                        "registration_already_complete".to_string(),
                        "Registration already completed".to_string(),
                    ),
                    DomainError::TokenServiceError(msg) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "token_service_error".to_string(),
                        msg,
                    ),
                    DomainError::EventError(msg) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "event_error".to_string(),
                        msg,
                    ),
                    DomainError::TokenNotFound => (
                        StatusCode::UNAUTHORIZED,
                        "token_not_found".to_string(),
                        "Token not found".to_string(),
                    ),
                }
            }
            ApiError::Command(cmd_error) => match cmd_error {
                CommandError::Validation { code, message } => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    code.clone(),
                    message.clone(),
                ),
                CommandError::Authentication { code, message } => {
                    (StatusCode::UNAUTHORIZED, code.clone(), message.clone())
                }
                CommandError::Business { code, message } => {
                    (StatusCode::BAD_REQUEST, code.clone(), message.clone())
                }
                CommandError::Infrastructure { code, message } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    code.clone(),
                    message.clone(),
                ),
                CommandError::Timeout { code, message } => {
                    (StatusCode::REQUEST_TIMEOUT, code.clone(), message.clone())
                }
                CommandError::RetryExhausted { code, message } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    code.clone(),
                    message.clone(),
                ),
            },
            ApiError::User(user_error) => {
                match user_error {
                    UserError::RepositoryError(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "repository_error".to_string(),
                        "Internal repository error".to_string(),
                    ),
                    UserError::TokenServiceError(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "token_service_error".to_string(),
                        "Token service error".to_string(),
                    ),
                    UserError::UserNotFound => (
                        StatusCode::NOT_FOUND,
                        "user_not_found".to_string(),
                        "User not found".to_string(),
                    ),
                    UserError::InvalidToken => (
                        StatusCode::UNAUTHORIZED,
                        "invalid_token".to_string(),
                        "Invalid token".to_string(),
                    ),
                    UserError::TokenExpired => (
                        StatusCode::UNAUTHORIZED,
                        "token_expired".to_string(),
                        "Token expired".to_string(),
                    ),
                    UserError::DomainError(domain_error) => {
                        // Delegate to domain error handling
                        let domain_api_error = ApiError::Domain(domain_error.clone());
                        return domain_api_error.into_response();
                    }
                }
            }
            ApiError::Token(token_error) => {
                match token_error {
                    TokenError::RepositoryError(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "repository_error".to_string(),
                        "Repository error".to_string(),
                    ),
                    TokenError::TokenServiceError(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "token_service_error".to_string(),
                        "Token service error".to_string(),
                    ),
                    TokenError::TokenNotFound => (
                        StatusCode::UNAUTHORIZED,
                        "token_not_found".to_string(),
                        "Refresh token not found".to_string(),
                    ),
                    TokenError::TokenInvalid => (
                        StatusCode::UNAUTHORIZED,
                        "token_invalid".to_string(),
                        "Refresh token is invalid".to_string(),
                    ),
                    TokenError::TokenExpired => (
                        StatusCode::UNAUTHORIZED,
                        "token_expired".to_string(),
                        "Refresh token is expired".to_string(),
                    ),
                    TokenError::DomainError(domain_error) => {
                        // Delegate to domain error handling
                        let domain_api_error = ApiError::Domain(domain_error.clone());
                        return domain_api_error.into_response();
                    }
                }
            }
            ApiError::AuthenticationRequired => (
                StatusCode::UNAUTHORIZED,
                "authentication_required".to_string(),
                "Authentication required".to_string(),
            ),
            ApiError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, "invalid_request".to_string(), msg)
            }
            ApiError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_server_error".to_string(),
                msg,
            ),
        };

        let body = Json(UniformErrorResponse {
            error: ErrorDetails {
                error_code,
                message,
                status: status.as_u16(),
            },
        });

        (status, body).into_response()
    }
}

// Helper functions for creating specific OAuth errors
impl AuthError {
    pub fn oauth_invalid_provider(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_provider".to_string(),
            message: "Invalid provider".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_invalid_authorization_header(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_authorization_header".to_string(),
            message: "Invalid Authorization header".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_invalid_token(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_token".to_string(),
            message: "Invalid or expired token".to_string(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn oauth_state_encoding_failed(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "state_encoding_failed".to_string(),
            message: "Failed to create OAuth state".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn oauth_url_generation_failed(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "url_generation_failed".to_string(),
            message: "Failed to generate authorization URL".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn oauth_invalid_url(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_url".to_string(),
            message: "Invalid authorization URL".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn oauth_missing_code(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "missing_code".to_string(),
            message: "Missing code parameter".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_invalid_state(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_state".to_string(),
            message: "Invalid state parameter".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_missing_state(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "missing_state".to_string(),
            message: "Missing state parameter".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_invalid_state_operation(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_state_operation".to_string(),
            message: "Invalid operation in state".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_provider_error(operation: &str, error: String, description: String) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: error,
            message: description,
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn oauth_login_failed(operation: &str, command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Business { code, .. } if code == "authentication_failed" => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Validation { code, message } => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            // Handle OAuth provider errors (invalid codes, user rejection, etc.) as authentication failures
            CommandError::Infrastructure { code, .. } if code == "provider_error" => Self::OAuth {
                operation: operation.to_string(),
                error_code: "authentication_failed".to_string(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            // Handle retry exhausted errors that originated from OAuth provider failures
            CommandError::RetryExhausted { message, .. }
                if message.contains("provider_error") || message.contains("OAuth") =>
            {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            // Handle other OAuth-related infrastructure errors as authentication failures
            CommandError::Infrastructure { code, .. }
                if code.contains("oauth") || code.contains("provider") =>
            {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Business { code, .. } => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Invalid email or password".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            _ => Self::OAuth {
                operation: operation.to_string(),
                error_code: "login_failed".to_string(),
                message: "Login failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    pub fn oauth_link_failed(
        operation: &str,
        command_error: &CommandError,
        provider: &str,
    ) -> Self {
        match command_error {
            CommandError::Business { code, .. } if code == "provider_already_linked_same_user" => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: code.clone(),
                    message: format!("{} is already linked to your account", provider),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, .. } if code == "provider_already_linked" => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: code.clone(),
                    message: format!(
                        "This {} account is already linked to another user",
                        provider
                    ),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, .. } if code == "authentication_failed" => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Business { code, .. } if code == "user_not_found" => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: "User not found".to_string(),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Validation { code, message } => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            // Handle OAuth provider errors (invalid codes, user rejection, etc.) as authentication failures
            CommandError::Infrastructure { code, .. } if code == "provider_error" => Self::OAuth {
                operation: operation.to_string(),
                error_code: "authentication_failed".to_string(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            // Handle retry exhausted errors that originated from OAuth provider failures
            CommandError::RetryExhausted { message, .. }
                if message.contains("provider_error") || message.contains("OAuth") =>
            {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            // Handle other OAuth-related infrastructure errors as authentication failures
            CommandError::Infrastructure { code, .. }
                if code.contains("oauth") || code.contains("provider") =>
            {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            _ => Self::OAuth {
                operation: operation.to_string(),
                error_code: "link_failed".to_string(),
                message: "Link provider failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    pub fn oauth_start_failed(command_error: &CommandError, provider: &str) -> Self {
        match command_error {
            CommandError::Validation { code, message } => Self::OAuth {
                operation: "oauth_start".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } if code == "provider_not_supported" => {
                Self::OAuth {
                    operation: "oauth_start".to_string(),
                    error_code: code.clone(),
                    message: format!("Provider {} not supported", provider),
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                }
            }
            _ => Self::OAuth {
                operation: "oauth_start".to_string(),
                error_code: "oauth_start_failed".to_string(),
                message: "Failed to generate OAuth start URL".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    pub fn link_provider_failed(command_error: &CommandError, provider: &str) -> Self {
        match command_error {
            CommandError::Business { code, .. } if code == "provider_already_linked_same_user" => {
                Self::OAuth {
                    operation: "link_provider".to_string(),
                    error_code: code.clone(),
                    message: format!("{} is already linked to your account", provider),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, .. } if code == "provider_already_linked" => {
                Self::OAuth {
                    operation: "link_provider".to_string(),
                    error_code: code.clone(),
                    message: format!(
                        "This {} account is already linked to another user",
                        provider
                    ),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, .. } if code == "business_rule_violation" => {
                Self::OAuth {
                    operation: "link_provider".to_string(),
                    error_code: code.clone(),
                    message: "Cannot relink provider that is not currently linked".to_string(),
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                }
            }
            CommandError::Business { code, .. } if code == "authentication_failed" => Self::OAuth {
                operation: "link_provider".to_string(),
                error_code: code.clone(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Business { code, .. } if code == "user_not_found" => Self::OAuth {
                operation: "link_provider".to_string(),
                error_code: code.clone(),
                message: "User not found".to_string(),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Validation { code, message } => Self::OAuth {
                operation: "link_provider".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            _ => Self::OAuth {
                operation: "link_provider".to_string(),
                error_code: "link_provider_failed".to_string(),
                message: "Failed to link provider".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Email/password signup failed
    pub fn signup_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } if code == "user_already_exists" => {
                AuthError::OAuth {
                    operation: "signup".to_string(),
                    error_code: code.clone(),
                    message: "User with this email already exists".to_string(),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, message } => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: "signup_failed".to_string(),
                message: "Signup failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Email/password login failed
    pub fn login_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_credentials" => {
                AuthError::OAuth {
                    operation: "login".to_string(),
                    error_code: code.clone(),
                    message: "Invalid email or password".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Business { code, .. } if code == "email_not_verified" => {
                AuthError::OAuth {
                    operation: "login".to_string(),
                    error_code: code.clone(),
                    message: "Please verify your email address before logging in".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Invalid email or password".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "login_failed".to_string(),
                message: "Login failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Email verification failed
    pub fn verification_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_verification_token" => {
                AuthError::OAuth {
                    operation: "verify".to_string(),
                    error_code: code.clone(),
                    message: "Invalid or expired verification token".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Business { code, .. } if code == "email_not_found" => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: code.clone(),
                message: "Verification request not found".to_string(),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Business { code, .. } if code == "email_already_verified" => {
                AuthError::OAuth {
                    operation: "verify".to_string(),
                    error_code: code.clone(),
                    message: "Email is already verified".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, message } => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "verification_failed".to_string(),
                message: "Email verification failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Provider token failed
    pub fn provider_token_failed(command_error: &CommandError, provider: &str) -> Self {
        match command_error {
            CommandError::Authentication { code, .. } if code == "authentication_failed" => {
                Self::OAuth {
                    operation: "internal_token".to_string(),
                    error_code: code.clone(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Validation { code, .. } if code == "provider_not_supported" => {
                Self::OAuth {
                    operation: "internal_token".to_string(),
                    error_code: code.clone(),
                    message: format!("Unsupported provider: {}", provider),
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                }
            }
            CommandError::Business { code, .. } if code == "no_token_for_provider" => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: format!("No token available for the user and provider {}", provider),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Authentication { code, .. } if code == "user_not_found" => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Validation { code, message } => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: "token_retrieval_failed".to_string(),
                message: "Failed to retrieve provider token".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Registration failed
    pub fn registration_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_token" => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Invalid registration token signature".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_expired" => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Registration token has expired".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } if code == "username_taken" => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Username is already taken".to_string(),
                status: StatusCode::CONFLICT,
            },
            CommandError::Business { code, .. } if code == "user_not_found" => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Registration session not found".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "invalid_username" => {
                AuthError::OAuth {
                    operation: "complete_registration".to_string(),
                    error_code: code.clone(),
                    message: "Invalid username format".to_string(),
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                }
            }
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, message } => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "complete_registration".to_string(),
                error_code: "registration_failed".to_string(),
                message: "Registration completion failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Username check failed
    pub fn username_check_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_username" => {
                AuthError::OAuth {
                    operation: "check_username".to_string(),
                    error_code: code.clone(),
                    message: "Invalid username format".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "check_username".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "check_username".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "check_username".to_string(),
                error_code: "username_check_failed".to_string(),
                message: "Username availability check failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Password reset request failed
    pub fn password_reset_request_failed(command_error: &CommandError) -> Self {
        match command_error {
            // Anti-enumeration: Always return 200 OK for reset requests regardless of error
            _ => AuthError::OAuth {
                operation: "password_reset_request".to_string(),
                error_code: "success".to_string(),
                message: "If a matching account was found, a password reset email has been sent".to_string(),
                status: StatusCode::OK,
            },
        }
    }

    /// Password reset token validation failed
    pub fn password_reset_validate_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_token" => AuthError::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_expired" => AuthError::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_already_used" => AuthError::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: "validation_failed".to_string(),
                message: "Token validation failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Password reset confirm failed (unauthenticated flow)
    pub fn password_reset_confirm_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_token" => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_expired" => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_already_used" => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "validation_failed" => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Password does not meet security requirements".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, message } => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: message.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: "reset_failed".to_string(),
                message: "Password reset failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Password reset authenticated failed
    pub fn password_reset_authenticated_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "incorrect_current_password" => AuthError::OAuth {
                operation: "password_reset_authenticated".to_string(),
                error_code: code.clone(),
                message: "Current password is incorrect".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "validation_failed" => AuthError::OAuth {
                operation: "password_reset_authenticated".to_string(),
                error_code: code.clone(),
                message: "Password validation failed".to_string(),
                status: StatusCode::UNPROCESSABLE_ENTITY,
            },
            CommandError::Business { code, .. } if code == "anti_enumeration_security" => AuthError::OAuth {
                operation: "password_reset_authenticated".to_string(),
                error_code: code.clone(),
                message: "Password reset request processed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => AuthError::OAuth {
                operation: "password_reset_authenticated".to_string(),
                error_code: "reset_failed".to_string(),
                message: "Password reset failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}
