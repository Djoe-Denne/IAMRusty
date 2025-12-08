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
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict { message: String },
    Validation { message: String },
    Internal { message: String },
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            HttpError::BadRequest { message } => (StatusCode::BAD_REQUEST, message),
            HttpError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            HttpError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            HttpError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            HttpError::Conflict { message } => (StatusCode::CONFLICT, message),
            HttpError::Validation { message } => (StatusCode::UNPROCESSABLE_ENTITY, message),
            HttpError::Internal { message } => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Map CommandError to HttpError
pub fn error_mapper(error: CommandError) -> HttpError {
    match error {
        CommandError::Validation { .. } => HttpError::Validation {
            message: error.to_string(),
        },
        CommandError::Business { .. } => {
            let msg = error.message();
            if msg.contains("not found") || msg.contains("Not found") {
                HttpError::NotFound
            } else if msg.contains("already exists") || msg.contains("Already exists") {
                HttpError::Conflict {
                    message: error.to_string(),
                }
            } else if msg.contains("permission") || msg.contains("Permission") {
                HttpError::Forbidden
            } else {
                HttpError::BadRequest {
                    message: error.to_string(),
                }
            }
        }
        CommandError::Infrastructure { .. } => HttpError::Internal {
            message: error.to_string(),
        },
        CommandError::RetryExhausted { .. } => HttpError::Internal {
            message: error.to_string(),
        },
        _ => HttpError::Internal {
            message: error.to_string(),
        },
    }
}

