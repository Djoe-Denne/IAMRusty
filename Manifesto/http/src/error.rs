use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rustycog_command::CommandError;
use serde_json::json;

#[derive(Debug)]
pub enum HttpError {
    BadRequest { message: String },
    Unauthorized { message: String },
    Forbidden { message: String },
    NotFound { message: String },
    Conflict { message: String },
    Validation { message: String },
    Internal { message: String },
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::BadRequest { message } => (StatusCode::BAD_REQUEST, message),
            Self::Unauthorized { message } => (StatusCode::UNAUTHORIZED, message),
            Self::Forbidden { message } => (StatusCode::FORBIDDEN, message),
            Self::NotFound { message } => (StatusCode::NOT_FOUND, message),
            Self::Conflict { message } => (StatusCode::CONFLICT, message),
            Self::Validation { message } => (StatusCode::UNPROCESSABLE_ENTITY, message),
            Self::Internal { message } => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Map `CommandError` to `HttpError`
#[must_use]
pub fn error_mapper(error: CommandError) -> HttpError {
    match error {
        CommandError::Validation { message, .. } => HttpError::Validation { message },
        CommandError::Authentication { code, message } => match code.as_str() {
            "permission_denied" => HttpError::Forbidden { message },
            _ => HttpError::Unauthorized { message },
        },
        CommandError::Business { code, message } => match code.as_str() {
            "not_found" => HttpError::NotFound { message },
            "already_exists" => HttpError::Conflict { message },
            "permission_denied" | "forbidden" => HttpError::Forbidden { message },
            _ => HttpError::BadRequest { message },
        },
        CommandError::Infrastructure { message, .. }
        | CommandError::RetryExhausted { message, .. }
        | CommandError::Timeout { message, .. } => HttpError::Internal { message },
    }
}
