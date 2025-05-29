use axum::{http::StatusCode, response::{Response, IntoResponse}, Json, extract::rejection::JsonRejection};
use serde::Serialize;
use domain::error::DomainError;
use application::error::ApplicationError;
use application::command::CommandError;
use application::usecase::{user::UserError, token::TokenError};
use thiserror::Error;
use validator::ValidationErrors as ValidatorValidationErrors;

/// Uniform error response structure for all API errors
#[derive(Debug, Serialize)]
pub struct UniformErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub error_code: String,
    pub message: String,
    pub status: u16,
}

/// Custom validation error for uniform error format
#[derive(Debug)]
pub struct ValidationError {
    pub error_code: String,
    pub message: String,
    pub status: StatusCode,
}

impl ValidationError {
    pub fn new(error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error_code: error_code.into(),
            message: message.into(),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
    
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        let body = Json(UniformErrorResponse {
            error: ErrorDetails {
                error_code: self.error_code,
                message: self.message,
                status: self.status.as_u16(),
            },
        });
        (self.status, body).into_response()
    }
}

/// Convert validator ValidationErrors to our uniform format
impl From<ValidatorValidationErrors> for ValidationError {
    fn from(errors: ValidatorValidationErrors) -> Self {
        // Extract the first validation error for a clean message
        let (field, error_info) = errors.errors()
            .iter()
            .next()
            .map(|(field, error_kind)| {
                let first_error = match error_kind {
                    validator::ValidationErrorsKind::Field(errors) => {
                        errors.first().map(|e| (e.code.as_ref(), e.message.as_deref()))
                    }
                    _ => None,
                };
                (field.as_ref(), first_error)
            })
            .unwrap_or(("unknown", None));

        let (error_code, message) = if let Some((code, msg)) = error_info {
            let formatted_message = msg.map(|s| s.to_string()).unwrap_or_else(|| {
                match code {
                    "empty_string" | "empty_password" | "empty_email" | "empty_username" => {
                        format!("{} is required", field.replace('_', " "))
                    }
                    "invalid_email_format" => "Invalid email format".to_string(),
                    "password_too_short" => "Password must be at least 8 characters long".to_string(),
                    "password_needs_letter" => "Password must contain at least one letter".to_string(),
                    "password_needs_digit" => "Password must contain at least one number".to_string(),
                    "password_too_common" => "Password is too common, please choose a stronger password".to_string(),
                    "invalid_username_format" => "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
                    "email_too_long" => "Email address is too long".to_string(),
                    "password_too_long" => "Password is too long".to_string(),
                    _ => format!("Invalid {}", field.replace('_', " ")),
                }
            });
            (format!("validation_{}", code), formatted_message)
        } else {
            ("validation_failed".to_string(), "Validation failed".to_string())
        };

        ValidationError::new(error_code, message)
    }
}

/// Convert JSON parsing errors to our uniform format
impl From<JsonRejection> for ValidationError {
    fn from(_rejection: JsonRejection) -> Self {
        ValidationError::new(
            "invalid_json",
            "Invalid JSON format in request body"
        ).with_status(StatusCode::BAD_REQUEST)
    }
}

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
            AuthError::OAuth { operation: _operation, error_code, message, status } => {
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
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::Domain(domain_error) => {
                match domain_error {
                    DomainError::UserNotFound => {
                        (StatusCode::NOT_FOUND, "user_not_found".to_string(), "User not found".to_string())
                    }
                    DomainError::ProviderNotSupported(msg) => {
                        (StatusCode::BAD_REQUEST, "provider_not_supported".to_string(), msg)
                    }
                    DomainError::InvalidToken => {
                        (StatusCode::UNAUTHORIZED, "invalid_token".to_string(), "Invalid token".to_string())
                    }
                    DomainError::TokenExpired => {
                        (StatusCode::UNAUTHORIZED, "token_expired".to_string(), "Token expired".to_string())
                    }
                    DomainError::AuthorizationError(msg) => {
                        (StatusCode::UNAUTHORIZED, "authorization_error".to_string(), msg)
                    }
                    DomainError::OAuth2Error(msg) => {
                        (StatusCode::BAD_REQUEST, "oauth2_error".to_string(), msg)
                    }
                    DomainError::UserProfileError(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "user_profile_error".to_string(), msg)
                    }
                    DomainError::NoTokenForProvider(provider, user) => {
                        (StatusCode::NOT_FOUND, "no_token_for_provider".to_string(), format!("No token found for provider {} and user {}", provider, user))
                    }
                    DomainError::TokenGenerationFailed(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "token_generation_failed".to_string(), msg)
                    }
                    DomainError::TokenValidationFailed(msg) => {
                        (StatusCode::UNAUTHORIZED, "token_validation_failed".to_string(), msg)
                    }
                    DomainError::RepositoryError(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "repository_error".to_string(), msg)
                    }
                }
            }
            ApiError::Application(app_error) => {
                match app_error {
                    ApplicationError::Domain(domain_error) => {
                        // Re-use domain error mapping
                        return ApiError::Domain(domain_error).into_response();
                    }
                    ApplicationError::Repository(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "repository_error".to_string(), msg)
                    }
                    ApplicationError::Service(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "service_error".to_string(), msg)
                    }
                    ApplicationError::OAuth2(msg) => {
                        (StatusCode::BAD_REQUEST, "oauth2_error".to_string(), msg)
                    }
                    ApplicationError::Token(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "token_error".to_string(), msg)
                    }
                    ApplicationError::UserProfile(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "user_profile_error".to_string(), msg)
                    }
                }
            }
            ApiError::Command(cmd_error) => {
                match cmd_error {
                    CommandError::Validation(msg) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "validation_error".to_string(), msg)
                    }
                    CommandError::Authentication(msg) => {
                        (StatusCode::UNAUTHORIZED, "authentication_error".to_string(), msg)
                    }
                    CommandError::Business(msg) => {
                        (StatusCode::BAD_REQUEST, "business_error".to_string(), msg)
                    }
                    CommandError::Infrastructure(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "infrastructure_error".to_string(), msg)
                    }
                    CommandError::Timeout => {
                        (StatusCode::REQUEST_TIMEOUT, "command_timeout".to_string(), "Command execution timeout".to_string())
                    }
                    CommandError::RetryExhausted(msg) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "retry_exhausted".to_string(), msg)
                    }
                }
            }
            ApiError::User(user_error) => {
                match user_error {
                    UserError::RepositoryError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "repository_error".to_string(), "Internal repository error".to_string())
                    }
                    UserError::TokenServiceError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "token_service_error".to_string(), "Token service error".to_string())
                    }
                    UserError::UserNotFound => {
                        (StatusCode::NOT_FOUND, "user_not_found".to_string(), "User not found".to_string())
                    }
                    UserError::InvalidToken => {
                        (StatusCode::UNAUTHORIZED, "invalid_token".to_string(), "Invalid token".to_string())
                    }
                    UserError::TokenExpired => {
                        (StatusCode::UNAUTHORIZED, "token_expired".to_string(), "Token expired".to_string())
                    }
                }
            }
            ApiError::Token(token_error) => {
                match token_error {
                    TokenError::RepositoryError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "repository_error".to_string(), "Repository error".to_string())
                    }
                    TokenError::TokenServiceError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "token_service_error".to_string(), "Token service error".to_string())
                    }
                    TokenError::TokenNotFound => {
                        (StatusCode::UNAUTHORIZED, "token_not_found".to_string(), "Refresh token not found".to_string())
                    }
                    TokenError::TokenInvalid => {
                        (StatusCode::UNAUTHORIZED, "token_invalid".to_string(), "Refresh token is invalid".to_string())
                    }
                    TokenError::TokenExpired => {
                        (StatusCode::UNAUTHORIZED, "token_expired".to_string(), "Refresh token is expired".to_string())
                    }
                }
            }
            ApiError::AuthenticationRequired => {
                (StatusCode::UNAUTHORIZED, "authentication_required".to_string(), "Authentication required".to_string())
            }
            ApiError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, "invalid_request".to_string(), msg)
            }
            ApiError::InternalServerError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_server_error".to_string(), msg)
            }
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
            CommandError::Business(_msg) => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "invalid_credentials".to_string(),
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

    /// Email/password signup failed
    pub fn signup_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation(msg) => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: "validation_failed".to_string(),
                message: msg.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business(msg) if msg.contains("User already exists") => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: "user_already_exists".to_string(),
                message: "User with this email already exists".to_string(),
                status: StatusCode::CONFLICT,
            },
            CommandError::Business(msg) => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: "signup_failed".to_string(),
                message: msg.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure(_) => AuthError::OAuth {
                operation: "signup".to_string(),
                error_code: "internal_error".to_string(),
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
            CommandError::Validation(msg) if msg.contains("Invalid credentials") => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "invalid_credentials".to_string(),
                message: "Invalid email or password".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Business(msg) if msg.contains("Email not verified") => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "email_not_verified".to_string(),
                message: "Please verify your email address before logging in".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Validation(msg) => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "validation_failed".to_string(),
                message: msg.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business(_msg) => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "invalid_credentials".to_string(),
                message: "Invalid email or password".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Infrastructure(_) => AuthError::OAuth {
                operation: "login".to_string(),
                error_code: "internal_error".to_string(),
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
            CommandError::Validation(msg) if msg.contains("Invalid or expired verification token") => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "invalid_token".to_string(),
                message: "Invalid or expired verification token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business(msg) if msg.contains("Invalid verification request") => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "not_found".to_string(),
                message: "Verification request not found".to_string(),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Business(msg) if msg.contains("Email is already verified") => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "already_verified".to_string(),
                message: "Email is already verified".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation(msg) => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "validation_failed".to_string(),
                message: msg.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business(msg) => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "verification_failed".to_string(),
                message: msg.clone(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure(_) => AuthError::OAuth {
                operation: "verify".to_string(),
                error_code: "internal_error".to_string(),
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
} 