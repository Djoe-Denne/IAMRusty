//! HTTP errors for enrollment and session.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use lazaret_application::InvokeError;
use lazaret_domain::IdentityError;
use serde::Serialize;

/// JSON error body.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// HTTP mapping for identity failures.
pub enum IdentityHttpError {
    /// 400
    BadRequest(String),
    /// 401
    Unauthorized(String),
    /// 409
    Conflict(String),
    /// 502
    BadGateway(String),
    /// 500
    Internal(String),
}

impl IdentityHttpError {
    /// 400 helper for malformed enroll bodies.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }
}

impl From<IdentityError> for IdentityHttpError {
    fn from(value: IdentityError) -> Self {
        match value {
            IdentityError::PrivateKeyNotAccepted
            | IdentityError::InvalidCsr(_)
            | IdentityError::InvalidIdentity
            | IdentityError::RegistryFull => Self::BadRequest(value.to_string()),
            IdentityError::InvalidCertificate
            | IdentityError::NotEnrolled
            | IdentityError::InvalidSession
            | IdentityError::UserClaimForbidden
            | IdentityError::MissingClientCertificate => Self::Unauthorized(value.to_string()),
            IdentityError::BindingAlreadyEnrolled => Self::Conflict(value.to_string()),
            IdentityError::ConsultFailed => Self::BadGateway(value.to_string()),
            IdentityError::SigningMaterial(_)
            | IdentityError::CaFailure(_)
            | IdentityError::EnrollmentStore(_) => Self::Internal(value.to_string()),
        }
    }
}

impl IntoResponse for IdentityHttpError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::BadGateway(message) => (StatusCode::BAD_GATEWAY, message),
            Self::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

/// HTTP mapping for [`InvokeError`].
pub enum InvokeHttpError {
    /// 400
    BadRequest(String),
    /// 401
    Unauthorized,
    /// 403
    Forbidden(String),
    /// 502/503 consult
    BadGateway(String),
    /// 413
    PayloadTooLarge,
    /// 500
    Failed,
}

impl From<InvokeError> for InvokeHttpError {
    fn from(value: InvokeError) -> Self {
        match value {
            InvokeError::Unauthorized => Self::Unauthorized,
            InvokeError::Forbidden(reason) => Self::Forbidden(reason.to_string()),
            InvokeError::BadRequest(message) => Self::BadRequest(message),
            InvokeError::ConsultFailed => Self::BadGateway("grant consult failed".to_owned()),
            InvokeError::PayloadTooLarge => Self::PayloadTooLarge,
            InvokeError::Failed => Self::Failed,
        }
    }
}

impl IntoResponse for InvokeHttpError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_owned()),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            Self::BadGateway(message) => (StatusCode::BAD_GATEWAY, message),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload too large".to_owned(),
            ),
            Self::Failed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "operation failed".to_owned(),
            ),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazaret_domain::GrantDenyReason;

    #[test]
    fn forbidden_json_uses_snake_case_not_debug() {
        let error =
            InvokeHttpError::from(InvokeError::Forbidden(GrantDenyReason::ComponentInactive));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let InvokeHttpError::Forbidden(message) =
            InvokeHttpError::from(InvokeError::Forbidden(GrantDenyReason::ComponentInactive))
        else {
            panic!("expected forbidden mapping");
        };
        assert_eq!(message, "component_inactive");
        assert!(!message.contains("ComponentInactive"));
        let body = serde_json::to_value(ErrorBody {
            error: message.clone(),
        })
        .expect("json");
        assert_eq!(body["error"], "component_inactive");
        assert!(!body.to_string().contains("ComponentInactive"));
    }
}
