use axum::{http::StatusCode, response::{Response, IntoResponse}, Json};
use serde_json::json;
use domain::error::DomainError;
use application::error::ApplicationError;
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
                },
                ApplicationError::Repository(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ApplicationError::Service(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ApplicationError::OAuth2(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                ApplicationError::Token(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ApplicationError::UserProfile(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
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