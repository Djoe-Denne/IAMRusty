use axum::{http::StatusCode, response::{Response, IntoResponse}, Json};
use serde_json::json;
use serde::Serialize;
use domain::error::DomainError;
use application::error::ApplicationError;
use application::command::CommandError;
use application::usecase::{user::UserError, token::TokenError};
use thiserror::Error;

/// API errors
#[derive(Debug, Error)]
pub enum ApiError {
    /// Domain error
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// Application error
    #[error(transparent)]
    Application(#[from] ApplicationError),

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

/// OAuth-specific error response for auth endpoints
#[derive(Debug, Serialize)]
pub struct OAuthErrorResponse {
    /// Operation type
    pub operation: String,
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
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
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Link failed
    #[error("Link failed")]
    LinkFailed,

    /// General API error
    #[error(transparent)]
    Api(#[from] ApiError),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::OAuth { operation, error_code, message, status } => {
                let body = Json(OAuthErrorResponse {
                    operation,
                    error: error_code,
                    message,
                });
                (status, body).into_response()
            }
            AuthError::InvalidProvider => {
                let body = Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "invalid_provider".to_string(),
                    message: "Invalid provider".to_string(),
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidAuthorizationHeader(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "invalid_authorization_header".to_string(),
                    message: "Invalid Authorization header".to_string(),
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidToken(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "invalid_token".to_string(),
                    message: "Invalid or expired token".to_string(),
                });
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AuthError::StateEncodingFailed(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "state_encoding_failed".to_string(),
                    message: "Failed to create OAuth state".to_string(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::UrlGenerationFailed(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "url_generation_failed".to_string(),
                    message: "Failed to generate authorization URL".to_string(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::InvalidUrl(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "start".to_string(),
                    error: "invalid_url".to_string(),
                    message: "Invalid authorization URL".to_string(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::OAuthError(error, description) => {
                let body = Json(OAuthErrorResponse {
                    operation: "callback".to_string(),
                    error,
                    message: description,
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::MissingCode => {
                let body = Json(OAuthErrorResponse {
                    operation: "callback".to_string(),
                    error: "missing_code".to_string(),
                    message: "Missing code parameter".to_string(),
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidState(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "callback".to_string(),
                    error: "invalid_state".to_string(),
                    message: "Invalid state parameter".to_string(),
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::MissingState => {
                let body = Json(OAuthErrorResponse {
                    operation: "callback".to_string(),
                    error: "missing_state".to_string(),
                    message: "Missing state parameter".to_string(),
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::InvalidStateOperation => {
                let body = Json(OAuthErrorResponse {
                    operation: "callback".to_string(),
                    error: "invalid_state_operation".to_string(),
                    message: "Invalid operation in state".to_string(),
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::AuthenticationFailed(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "login".to_string(),
                    error: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                });
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AuthError::ValidationFailed(msg) => {
                let body = Json(OAuthErrorResponse {
                    operation: "login".to_string(),
                    error: "validation_failed".to_string(),
                    message: msg,
                });
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AuthError::LoginFailed => {
                let body = Json(OAuthErrorResponse {
                    operation: "login".to_string(),
                    error: "login_failed".to_string(),
                    message: "Login failed".to_string(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::ProviderAlreadyLinkedToSameUser(msg) => {
                let body = Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "provider_already_linked_to_same_user".to_string(),
                    message: msg,
                });
                (StatusCode::CONFLICT, body).into_response()
            }
            AuthError::ProviderAlreadyLinked(msg) => {
                let body = Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "provider_already_linked".to_string(),
                    message: msg,
                });
                (StatusCode::CONFLICT, body).into_response()
            }
            AuthError::UserNotFound(_) => {
                let body = Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "user_not_found".to_string(),
                    message: "User not found".to_string(),
                });
                (StatusCode::NOT_FOUND, body).into_response()
            }
            AuthError::LinkFailed => {
                let body = Json(OAuthErrorResponse {
                    operation: "link".to_string(),
                    error: "link_failed".to_string(),
                    message: "Link provider failed".to_string(),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AuthError::Api(api_error) => api_error.into_response(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::Domain(e) => match e {
                DomainError::UserNotFound => (StatusCode::NOT_FOUND, e.to_string()),
                DomainError::ProviderNotSupported(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                DomainError::InvalidToken => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::TokenExpired => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::AuthorizationError(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::OAuth2Error(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                DomainError::UserProfileError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                DomainError::NoTokenForProvider(_, _) => (StatusCode::NOT_FOUND, e.to_string()),
                DomainError::TokenGenerationFailed(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                DomainError::TokenValidationFailed(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                DomainError::RepositoryError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            ApiError::Application(e) => match e {
                ApplicationError::Domain(domain_err) => match domain_err {
                    DomainError::UserNotFound => (StatusCode::NOT_FOUND, e.to_string()),
                    DomainError::ProviderNotSupported(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                    DomainError::InvalidToken => (StatusCode::UNAUTHORIZED, e.to_string()),
                    DomainError::TokenExpired => (StatusCode::UNAUTHORIZED, e.to_string()),
                    DomainError::AuthorizationError(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                    DomainError::OAuth2Error(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                    DomainError::UserProfileError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                    DomainError::NoTokenForProvider(_, _) => (StatusCode::NOT_FOUND, e.to_string()),
                    DomainError::TokenGenerationFailed(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                    DomainError::TokenValidationFailed(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                    DomainError::RepositoryError(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                },
                ApplicationError::Repository(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ApplicationError::Service(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ApplicationError::OAuth2(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                ApplicationError::Token(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ApplicationError::UserProfile(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            ApiError::Command(e) => match e {
                CommandError::Business(msg) if msg.contains("Authentication failed") => {
                    (StatusCode::UNAUTHORIZED, "Authentication failed".to_string())
                }
                CommandError::Business(msg) if msg.contains("User not found") => {
                    (StatusCode::NOT_FOUND, "User not found".to_string())
                }
                CommandError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            ApiError::User(e) => match e {
                UserError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user".to_string()),
            },
            ApiError::Token(e) => match e {
                TokenError::TokenNotFound => (StatusCode::UNAUTHORIZED, "Invalid refresh token".to_string()),
                TokenError::TokenInvalid => (StatusCode::UNAUTHORIZED, "Invalid refresh token".to_string()),
                TokenError::TokenExpired => (StatusCode::UNAUTHORIZED, "Expired refresh token".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to refresh token".to_string()),
            },
            ApiError::AuthenticationRequired => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "status": status.as_u16(),
            }
        }));

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
            CommandError::Business(msg) if msg.contains("Authentication failed") => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Validation(msg) => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "validation_failed".to_string(),
                    message: msg.clone(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            _ => Self::OAuth {
                operation: operation.to_string(),
                error_code: "login_failed".to_string(),
                message: "Login failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    pub fn oauth_link_failed(operation: &str, command_error: &CommandError, provider: &str) -> Self {
        match command_error {
            CommandError::Business(msg) if msg.contains("already linked to your account") => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "provider_already_linked_to_same_user".to_string(),
                    message: format!("{} is already linked to your account", provider),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business(msg) if msg.contains("already linked to another user") => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "provider_already_linked".to_string(),
                    message: format!("This {} account is already linked to another user", provider),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business(msg) if msg.contains("Authentication failed") => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "authentication_failed".to_string(),
                    message: "Authentication failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            }
            CommandError::Business(msg) if msg.contains("User not found") => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "user_not_found".to_string(),
                    message: "User not found".to_string(),
                    status: StatusCode::NOT_FOUND,
                }
            }
            CommandError::Validation(msg) => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: "validation_failed".to_string(),
                    message: msg.clone(),
                    status: StatusCode::BAD_REQUEST,
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
} 