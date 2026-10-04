use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use iam_application::command::CommandError;
use iam_application::usecase::{token::TokenError, user::UserError};
use iam_domain::error::DomainError;
use rustycog::http::error::{ErrorDetails, UniformErrorResponse};
use thiserror::Error;

#[cfg(test)]
mod credential_redaction_tests {
    use super::*;

    const SENTINEL: &str = "SENTINEL-AUTH-CAUSE-CREDENTIAL";

    #[test]
    fn credential_bearing_auth_errors_have_safe_debug_and_display() {
        for error in [
            AuthError::InvalidToken(SENTINEL.into()),
            AuthError::InvalidState(SENTINEL.into()),
            AuthError::InvalidUrl(SENTINEL.into()),
            AuthError::OAuthError(SENTINEL.into(), SENTINEL.into()),
            AuthError::oauth_provider_error("callback", SENTINEL.into(), SENTINEL.into()),
            AuthError::RegistrationIncomplete {
                registration_token: SENTINEL.into(),
                message: "Registration incomplete".into(),
            },
        ] {
            assert!(!format!("{error:?}").contains(SENTINEL));
            assert!(!error.to_string().contains(SENTINEL));
        }
    }

    #[tokio::test]
    async fn public_errors_do_not_reflect_provider_or_command_credentials() {
        let cause = CommandError::Validation {
            code: "invalid_token".into(),
            message: SENTINEL.into(),
        };
        for response in [
            AuthError::oauth_provider_error("callback", SENTINEL.into(), SENTINEL.into())
                .into_response(),
            AuthError::oauth_login_failed("callback", &cause).into_response(),
            AuthError::password_reset_validate_failed(&cause).into_response(),
            ApiError::Domain(DomainError::OAuth2Error(SENTINEL.into())).into_response(),
        ] {
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            assert!(!String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains(SENTINEL));
        }
    }
}

#[cfg(test)]
mod signing_admission_mapping_tests {
    use super::*;
    use iam_domain::entity::signing_key::SigningKeyAdmissionReason;

    #[tokio::test]
    async fn admission_errors_keep_s10_status_and_authoritative_retry_delay() {
        for reason in [
            SigningKeyAdmissionReason::Capacity,
            SigningKeyAdmissionReason::TenantEpochLimit,
            SigningKeyAdmissionReason::ChurnRate,
            SigningKeyAdmissionReason::EpochConflict,
        ] {
            let status = if reason == SigningKeyAdmissionReason::EpochConflict {
                StatusCode::CONFLICT
            } else {
                StatusCode::TOO_MANY_REQUESTS
            };
            for retry in [None, Some(17)] {
                let response = ApiError::Domain(DomainError::SigningKeyAdmissionDenied {
                    reason: reason.clone(),
                    retry_after_seconds: retry,
                })
                .into_response();
                assert_eq!(response.status(), status);
                assert_eq!(
                    response
                        .headers()
                        .get(axum::http::header::RETRY_AFTER)
                        .map(|value| value.to_str().unwrap()),
                    if status == StatusCode::TOO_MANY_REQUESTS && retry.is_some() {
                        Some("17")
                    } else {
                        None
                    }
                );
                let bytes = axum::body::to_bytes(response.into_body(), 4096)
                    .await
                    .unwrap();
                let body = String::from_utf8(bytes.to_vec()).unwrap();
                assert!(body.contains("Signing key admission denied"));
                assert!(!body.contains(&format!("{reason:?}")));
            }
        }
    }

    #[test]
    fn invalid_signing_material_is_bad_request_not_unprocessable_entity() {
        let response = ApiError::Domain(DomainError::InvalidSigningKeyMaterial).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!response
            .headers()
            .contains_key(axum::http::header::RETRY_AFTER));
    }
}

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
#[derive(Error)]
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
    #[error("Invalid authorization header")]
    InvalidAuthorizationHeader(String),

    /// Invalid token
    #[error("Invalid token")]
    InvalidToken(String),

    /// State encoding failed
    #[error("State encoding failed")]
    StateEncodingFailed(String),

    /// URL generation failed
    #[error("URL generation failed")]
    UrlGenerationFailed(String),

    /// Invalid URL
    #[error("Invalid URL")]
    InvalidUrl(String),

    /// OAuth error from provider
    #[error("OAuth provider rejected the request")]
    OAuthError(String, String),

    /// Missing code parameter
    #[error("Missing code parameter")]
    MissingCode,

    /// Invalid state
    #[error("Invalid state")]
    InvalidState(String),

    /// Missing state parameter
    #[error("Missing state parameter")]
    MissingState,

    /// Invalid state operation
    #[error("Invalid state operation")]
    InvalidStateOperation,

    /// Authentication failed
    #[error("Authentication failed")]
    AuthenticationFailed(String),

    /// Validation failed
    #[error("Validation failed")]
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
    #[error("API operation failed")]
    Api(#[from] ApiError),

    /// Registration incomplete with token
    #[error("{message}")]
    RegistrationIncomplete {
        registration_token: String,
        message: String,
    },
}

// Raw causes and registration tokens must not escape through a debug event either.
impl std::fmt::Debug for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AuthError({:?})", std::mem::discriminant(self))
    }
}

fn uniform_error(
    status: StatusCode,
    error_code: impl Into<String>,
    message: impl Into<String>,
) -> Response {
    let body = Json(UniformErrorResponse {
        error: ErrorDetails {
            error_code: error_code.into(),
            message: message.into(),
            status: status.as_u16(),
        },
    });
    (status, body).into_response()
}

fn map_auth_protocol_error(error: AuthError) -> Response {
    match error {
        AuthError::OAuth {
            error_code,
            message,
            status,
            ..
        } => uniform_error(status, error_code, message),
        AuthError::InvalidProvider => uniform_error(
            StatusCode::BAD_REQUEST,
            "invalid_provider",
            "Invalid provider",
        ),
        AuthError::InvalidAuthorizationHeader(_) => uniform_error(
            StatusCode::BAD_REQUEST,
            "invalid_authorization_header",
            "Invalid Authorization header",
        ),
        AuthError::InvalidToken(_) => uniform_error(
            StatusCode::UNAUTHORIZED,
            "invalid_token",
            "Invalid or expired token",
        ),
        AuthError::StateEncodingFailed(_) => uniform_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "state_encoding_failed",
            "Failed to create OAuth state",
        ),
        AuthError::UrlGenerationFailed(_) => uniform_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "url_generation_failed",
            "Failed to generate authorization URL",
        ),
        AuthError::InvalidUrl(_) => uniform_error(
            StatusCode::BAD_REQUEST,
            "invalid_url",
            "Invalid URL in OAuth callback",
        ),
        AuthError::OAuthError(_, _) => uniform_error(
            StatusCode::BAD_REQUEST,
            "oauth_provider_error",
            "OAuth provider rejected the request",
        ),
        AuthError::MissingCode => uniform_error(
            StatusCode::BAD_REQUEST,
            "missing_code",
            "Missing authorization code",
        ),
        AuthError::InvalidState(_) => uniform_error(
            StatusCode::BAD_REQUEST,
            "invalid_state",
            "Invalid OAuth state parameter",
        ),
        AuthError::MissingState => uniform_error(
            StatusCode::BAD_REQUEST,
            "missing_state",
            "Missing OAuth state parameter",
        ),
        AuthError::InvalidStateOperation => uniform_error(
            StatusCode::BAD_REQUEST,
            "invalid_state_operation",
            "Invalid OAuth state operation",
        ),
        _ => unreachable!("non-protocol AuthError passed to map_auth_protocol_error"),
    }
}

fn map_auth_session_error(error: AuthError) -> Response {
    match error {
        AuthError::AuthenticationFailed(_) => uniform_error(
            StatusCode::UNAUTHORIZED,
            "authentication_failed",
            "Authentication failed",
        ),
        AuthError::ValidationFailed(_) => uniform_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Request validation failed",
        ),
        AuthError::LoginFailed => {
            uniform_error(StatusCode::UNAUTHORIZED, "login_failed", "Login failed")
        }
        AuthError::ProviderAlreadyLinkedToSameUser(provider) => uniform_error(
            StatusCode::CONFLICT,
            "provider_already_linked_same_user",
            format!("Provider {provider} is already linked to this user"),
        ),
        AuthError::ProviderAlreadyLinked(provider) => uniform_error(
            StatusCode::CONFLICT,
            "provider_already_linked",
            format!("Provider {provider} is already linked to another user"),
        ),
        AuthError::UserNotFound(user_id) => uniform_error(
            StatusCode::NOT_FOUND,
            "user_not_found",
            format!("User not found: {user_id}"),
        ),
        AuthError::LinkFailed => uniform_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "link_failed",
            "Failed to link provider",
        ),
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
        _ => unreachable!("protocol AuthError passed to map_auth_session_error"),
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            Self::OAuth { .. }
            | Self::InvalidProvider
            | Self::InvalidAuthorizationHeader(_)
            | Self::InvalidToken(_)
            | Self::StateEncodingFailed(_)
            | Self::UrlGenerationFailed(_)
            | Self::InvalidUrl(_)
            | Self::OAuthError(_, _)
            | Self::MissingCode
            | Self::InvalidState(_)
            | Self::MissingState
            | Self::InvalidStateOperation => map_auth_protocol_error(self),
            _ => map_auth_session_error(self),
        }
    }
}

fn map_domain_error(domain_error: DomainError) -> (StatusCode, String, String) {
    match domain_error {
        DomainError::UserNotFound => (
            StatusCode::NOT_FOUND,
            "user_not_found".into(),
            "User not found".into(),
        ),
        DomainError::ProviderNotSupported(msg) => (
            StatusCode::BAD_REQUEST,
            "provider_not_supported".into(),
            msg,
        ),
        DomainError::ConnectorNotConfigured(provider) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "connector_not_configured".into(),
            format!("IdP connector is not configured for this provider: {provider}"),
        ),
        DomainError::BusinessRuleViolation(msg) => (
            StatusCode::BAD_REQUEST,
            "business_rule_violation".into(),
            msg,
        ),
        DomainError::InvalidToken => (
            StatusCode::UNAUTHORIZED,
            "invalid_token".into(),
            "Invalid token".into(),
        ),
        DomainError::TokenExpired => (
            StatusCode::UNAUTHORIZED,
            "token_expired".into(),
            "Token expired".into(),
        ),
        DomainError::AuthorizationError(_) => (
            StatusCode::UNAUTHORIZED,
            "authorization_error".into(),
            "Authorization failed".into(),
        ),
        DomainError::OAuth2Error(_) => (
            StatusCode::BAD_REQUEST,
            "oauth2_error".into(),
            "OAuth operation failed".into(),
        ),
        DomainError::UserProfileError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "user_profile_error".into(),
            "User profile operation failed".into(),
        ),
        DomainError::NoTokenForProvider => (
            StatusCode::NOT_FOUND,
            "no_token_for_provider".into(),
            "No token found for provider and user".into(),
        ),
        DomainError::TokenGenerationFailed(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "token_generation_failed".into(),
            "Token generation failed".into(),
        ),
        DomainError::TokenValidationFailed(_) => (
            StatusCode::UNAUTHORIZED,
            "token_validation_failed".into(),
            "Token validation failed".into(),
        ),
        DomainError::RepositoryError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "repository_error".into(),
            "Repository operation failed".into(),
        ),
        DomainError::UsernameTaken => (
            StatusCode::CONFLICT,
            "username_taken".into(),
            "Username already taken".into(),
        ),
        DomainError::InvalidUsername => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_username".into(),
            "Invalid username format".into(),
        ),
        DomainError::RegistrationAlreadyComplete => (
            StatusCode::BAD_REQUEST,
            "registration_already_complete".into(),
            "Registration already completed".into(),
        ),
        DomainError::TokenServiceError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "token_service_error".into(),
            "Token service operation failed".into(),
        ),
        DomainError::EventError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event_error".into(),
            "Event operation failed".into(),
        ),
        DomainError::ExternalServiceError { .. } => (
            StatusCode::BAD_GATEWAY,
            "external_service_error".into(),
            "External service failed".into(),
        ),
        DomainError::TokenNotFound => (
            StatusCode::UNAUTHORIZED,
            "token_not_found".into(),
            "Token not found".into(),
        ),
        // Keep the generic API fallback aligned with the org-signer S-10 contract.
        DomainError::SigningKeyAdmissionDenied { reason, .. } => (
            if reason == iam_domain::entity::signing_key::SigningKeyAdmissionReason::EpochConflict {
                StatusCode::CONFLICT
            } else {
                StatusCode::TOO_MANY_REQUESTS
            },
            "signing_key_admission_denied".into(),
            "Signing key admission denied".into(),
        ),
        DomainError::InvalidSigningKeyMaterial => (
            StatusCode::BAD_REQUEST,
            "invalid_signing_key_material".into(),
            "Invalid signing key material".into(),
        ),
    }
}

fn map_command_error(cmd_error: CommandError) -> (StatusCode, String, String) {
    match cmd_error {
        CommandError::Validation { code, .. } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            code,
            "Request validation failed".into(),
        ),
        CommandError::Authentication { code, .. } => (
            StatusCode::UNAUTHORIZED,
            code,
            "Authentication failed".into(),
        ),
        CommandError::Business { code, .. } => {
            (StatusCode::BAD_REQUEST, code, "Operation rejected".into())
        }
        CommandError::Timeout { code, .. } => (
            StatusCode::REQUEST_TIMEOUT,
            code,
            "Operation timed out".into(),
        ),
        CommandError::Infrastructure { code, .. } | CommandError::RetryExhausted { code, .. } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            code,
            "Operation failed".into(),
        ),
    }
}

fn map_user_error(user_error: &UserError) -> (StatusCode, String, String) {
    match user_error {
        UserError::RepositoryError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "repository_error".into(),
            "Internal repository error".into(),
        ),
        UserError::TokenServiceError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "token_service_error".into(),
            "Token service error".into(),
        ),
        UserError::UserNotFound => (
            StatusCode::NOT_FOUND,
            "user_not_found".into(),
            "User not found".into(),
        ),
        UserError::InvalidToken => (
            StatusCode::UNAUTHORIZED,
            "invalid_token".into(),
            "Invalid token".into(),
        ),
        UserError::TokenExpired => (
            StatusCode::UNAUTHORIZED,
            "token_expired".into(),
            "Token expired".into(),
        ),
        UserError::DomainError(_) => unreachable!(),
    }
}

fn map_token_error(token_error: &TokenError) -> (StatusCode, String, String) {
    match token_error {
        TokenError::RepositoryError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "repository_error".into(),
            "Repository error".into(),
        ),
        TokenError::TokenServiceError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "token_service_error".into(),
            "Token service error".into(),
        ),
        TokenError::TokenNotFound => (
            StatusCode::UNAUTHORIZED,
            "token_not_found".into(),
            "Refresh token not found".into(),
        ),
        TokenError::TokenInvalid => (
            StatusCode::UNAUTHORIZED,
            "token_invalid".into(),
            "Refresh token is invalid".into(),
        ),
        TokenError::TokenExpired => (
            StatusCode::UNAUTHORIZED,
            "token_expired".into(),
            "Refresh token is expired".into(),
        ),
        TokenError::DomainError(_) => unreachable!(),
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry_after = match &self {
            Self::Domain(DomainError::SigningKeyAdmissionDenied {
                retry_after_seconds,
                ..
            }) => *retry_after_seconds,
            _ => None,
        };
        let (status, error_code, message) = match self {
            Self::Domain(domain_error) => map_domain_error(domain_error),
            Self::Command(cmd_error) => map_command_error(cmd_error),
            Self::User(UserError::DomainError(domain_error))
            | Self::Token(TokenError::DomainError(domain_error)) => {
                return Self::Domain(domain_error).into_response();
            }
            Self::User(user_error) => map_user_error(&user_error),
            Self::Token(token_error) => map_token_error(&token_error),
            Self::AuthenticationRequired => (
                StatusCode::UNAUTHORIZED,
                "authentication_required".into(),
                "Authentication required".into(),
            ),
            Self::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, "invalid_request".into(), msg),
            Self::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_server_error".into(),
                msg,
            ),
        };
        let mut response = uniform_error(status, error_code, message);
        // Only the authoritative DB-provided delay may become Retry-After.
        if status == StatusCode::TOO_MANY_REQUESTS {
            if let Some(retry) = retry_after {
                if let Ok(value) = retry.to_string().parse() {
                    response
                        .headers_mut()
                        .insert(axum::http::header::RETRY_AFTER, value);
                }
            }
        }
        response
    }
}

// Helper functions for creating specific OAuth errors
impl AuthError {
    #[must_use]
    pub fn oauth_invalid_provider(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_provider".to_string(),
            message: "Invalid provider".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_connector_not_configured(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "connector_not_configured".to_string(),
            message: "IdP connector is not configured for this provider".to_string(),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    #[must_use]
    pub fn oauth_invalid_authorization_header(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_authorization_header".to_string(),
            message: "Invalid Authorization header".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_invalid_token(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_token".to_string(),
            message: "Invalid or expired token".to_string(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    #[must_use]
    pub fn oauth_state_encoding_failed(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "state_encoding_failed".to_string(),
            message: "Failed to create OAuth state".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub fn oauth_url_generation_failed(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "url_generation_failed".to_string(),
            message: "Failed to generate authorization URL".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub fn oauth_invalid_url(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_url".to_string(),
            message: "Invalid authorization URL".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub fn oauth_missing_code(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "missing_code".to_string(),
            message: "Missing code parameter".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_invalid_state(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_state".to_string(),
            message: "Invalid state parameter".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_missing_state(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "missing_state".to_string(),
            message: "Missing state parameter".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_invalid_state_operation(operation: &str) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "invalid_state_operation".to_string(),
            message: "Invalid operation in state".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_provider_error(operation: &str, _error: String, _description: String) -> Self {
        Self::OAuth {
            operation: operation.to_string(),
            error_code: "oauth_provider_error".to_string(),
            message: "OAuth provider rejected the request".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    #[must_use]
    pub fn oauth_login_failed(operation: &str, command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Business { code, .. } if code == "authentication_failed" => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Validation { code, .. } if code == "connector_not_configured" => {
                Self::oauth_connector_not_configured(operation)
            }
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
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
            CommandError::Business { code, .. } => Self::OAuth {
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

    #[must_use]
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
                    message: format!("{provider} is already linked to your account"),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, .. } if code == "provider_already_linked" => {
                Self::OAuth {
                    operation: operation.to_string(),
                    error_code: code.clone(),
                    message: format!("This {provider} account is already linked to another user"),
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
            CommandError::Validation { code, .. } if code == "connector_not_configured" => {
                Self::oauth_connector_not_configured(operation)
            }
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: operation.to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
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

    #[must_use]
    pub fn oauth_start_failed(command_error: &CommandError, provider: &str) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "connector_not_configured" => {
                Self::oauth_connector_not_configured("oauth_start")
            }
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "oauth_start".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } if code == "provider_not_supported" => {
                Self::OAuth {
                    operation: "oauth_start".to_string(),
                    error_code: code.clone(),
                    message: format!("Provider {provider} not supported"),
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

    #[must_use]
    pub fn link_provider_failed(command_error: &CommandError, provider: &str) -> Self {
        match command_error {
            CommandError::Business { code, .. } if code == "provider_already_linked_same_user" => {
                Self::OAuth {
                    operation: "link_provider".to_string(),
                    error_code: code.clone(),
                    message: format!("{provider} is already linked to your account"),
                    status: StatusCode::CONFLICT,
                }
            }
            CommandError::Business { code, .. } if code == "provider_already_linked" => {
                Self::OAuth {
                    operation: "link_provider".to_string(),
                    error_code: code.clone(),
                    message: format!("This {provider} account is already linked to another user"),
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
            CommandError::Validation { code, .. } if code == "connector_not_configured" => {
                Self::oauth_connector_not_configured("link_provider")
            }
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "link_provider".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
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
    #[must_use]
    pub fn signup_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Business { code, .. } if code == "user_already_exists" => Self::OAuth {
                operation: "signup".to_string(),
                error_code: code.clone(),
                message: "User with this email already exists".to_string(),
                status: StatusCode::CONFLICT,
            },
            CommandError::Validation { code, .. } | CommandError::Business { code, .. } => {
                Self::OAuth {
                    operation: "signup".to_string(),
                    error_code: code.clone(),
                    message: "Request validation failed".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "signup".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "signup".to_string(),
                error_code: "signup_failed".to_string(),
                message: "Signup failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Email/password login failed
    #[must_use]
    pub fn login_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_credentials" => Self::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Invalid email or password".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Business { code, .. } if code == "email_not_verified" => Self::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Please verify your email address before logging in".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } => Self::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Invalid email or password".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "login".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "login".to_string(),
                error_code: "login_failed".to_string(),
                message: "Login failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Email verification failed
    #[must_use]
    pub fn verification_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_verification_token" => {
                Self::OAuth {
                    operation: "verify".to_string(),
                    error_code: code.clone(),
                    message: "Invalid or expired verification token".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Business { code, .. } if code == "email_not_found" => Self::OAuth {
                operation: "verify".to_string(),
                error_code: code.clone(),
                message: "Verification request not found".to_string(),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Business { code, .. } if code == "email_already_verified" => {
                Self::OAuth {
                    operation: "verify".to_string(),
                    error_code: code.clone(),
                    message: "Email is already verified".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Validation { code, .. } | CommandError::Business { code, .. } => {
                Self::OAuth {
                    operation: "verify".to_string(),
                    error_code: code.clone(),
                    message: "Request validation failed".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "verify".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "verify".to_string(),
                error_code: "verification_failed".to_string(),
                message: "Email verification failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Provider token failed
    #[must_use]
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
                    message: format!("Unsupported provider: {provider}"),
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                }
            }
            CommandError::Validation { code, .. } if code == "connector_not_configured" => {
                Self::oauth_connector_not_configured("internal_token")
            }
            CommandError::Business { code, .. } if code == "no_token_for_provider" => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: format!("No token available for the user and provider {provider}"),
                status: StatusCode::NOT_FOUND,
            },
            CommandError::Authentication { code, .. } if code == "user_not_found" => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: "Authentication failed".to_string(),
                status: StatusCode::UNAUTHORIZED,
            },
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "internal_token".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
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
    #[must_use]
    pub fn registration_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_token" => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Invalid registration token signature".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_expired" => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Registration token has expired".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Business { code, .. } if code == "username_taken" => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Username is already taken".to_string(),
                status: StatusCode::CONFLICT,
            },
            CommandError::Business { code, .. } if code == "user_not_found" => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Registration session not found".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "invalid_username" => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Invalid username format".to_string(),
                status: StatusCode::UNPROCESSABLE_ENTITY,
            },
            CommandError::Validation { code, .. } | CommandError::Business { code, .. } => {
                Self::OAuth {
                    operation: "complete_registration".to_string(),
                    error_code: code.clone(),
                    message: "Request validation failed".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "complete_registration".to_string(),
                error_code: "registration_failed".to_string(),
                message: "Registration completion failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Username check failed
    #[must_use]
    pub fn username_check_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_username" => Self::OAuth {
                operation: "check_username".to_string(),
                error_code: code.clone(),
                message: "Invalid username format".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "check_username".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "check_username".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "check_username".to_string(),
                error_code: "username_check_failed".to_string(),
                message: "Username availability check failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Password reset request failed
    #[must_use]
    pub fn password_reset_request_failed(_command_error: &CommandError) -> Self {
        Self::OAuth {
            operation: "password_reset_request".to_string(),
            error_code: "success".to_string(),
            message: "If a matching account was found, a password reset email has been sent"
                .to_string(),
            status: StatusCode::OK,
        }
    }

    /// Password reset token validation failed
    #[must_use]
    pub fn password_reset_validate_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_token" => Self::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_expired" => Self::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_already_used" => Self::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "password_reset_validate".to_string(),
                error_code: "validation_failed".to_string(),
                message: "Token validation failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Password reset confirm failed (unauthenticated flow)
    #[must_use]
    pub fn password_reset_confirm_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "invalid_token" => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_expired" => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "token_already_used" => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Invalid or expired reset token".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } if code == "validation_failed" => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Password does not meet security requirements".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Validation { code, .. } => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Request validation failed".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            CommandError::Infrastructure { code, .. } => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: code.clone(),
                message: "Internal server error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self::OAuth {
                operation: "password_reset_confirm".to_string(),
                error_code: "reset_failed".to_string(),
                message: "Password reset failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    /// Password reset authenticated failed
    #[must_use]
    pub fn password_reset_authenticated_failed(command_error: &CommandError) -> Self {
        match command_error {
            CommandError::Validation { code, .. } if code == "incorrect_current_password" => {
                Self::OAuth {
                    operation: "password_reset_authenticated".to_string(),
                    error_code: code.clone(),
                    message: "Current password is incorrect".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            }
            CommandError::Validation { code, .. } if code == "validation_failed" => Self::OAuth {
                operation: "password_reset_authenticated".to_string(),
                error_code: code.clone(),
                message: "Password validation failed".to_string(),
                status: StatusCode::UNPROCESSABLE_ENTITY,
            },
            CommandError::Business { code, .. } if code == "anti_enumeration_security" => {
                Self::OAuth {
                    operation: "password_reset_authenticated".to_string(),
                    error_code: code.clone(),
                    message: "Password reset request processed".to_string(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
            _ => Self::OAuth {
                operation: "password_reset_authenticated".to_string(),
                error_code: "reset_failed".to_string(),
                message: "Password reset failed".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}
