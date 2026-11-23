//! Authenticated account-owned TOTP enrollment routes.

use crate::{platform_user::PlatformUser, AuthError};
use axum::{
    extract::{rejection::JsonRejection, Path},
    http::{header::CACHE_CONTROL, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use iam_application::usecase::login::LoginUseCase;
use iam_domain::port::totp::TotpError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub const ENROLLMENT: &str = "/api/me/totp/enrollment";
pub const CONFIRM: &str = "/api/me/totp/enrollment/confirm";
pub const CANCEL: &str = "/api/me/totp/enrollment/{enrollment_id}";

#[derive(Serialize)]
struct EnrollmentResponse {
    enrollment_id: Uuid,
    otpauth_uri: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmRequest {
    enrollment_id: Uuid,
    current_password: String,
    code: String,
}

pub async fn begin(
    owner: PlatformUser,
    Extension(login): Extension<Arc<dyn LoginUseCase>>,
) -> Result<impl IntoResponse, AuthError> {
    let enrollment = login
        .begin_totp(owner.user_id)
        .await
        .map_err(|error| enrollment_error(&error))?;
    Ok((
        StatusCode::CREATED,
        [(CACHE_CONTROL, "no-store")],
        Json(EnrollmentResponse {
            enrollment_id: enrollment.id,
            otpauth_uri: enrollment.otpauth_uri,
        }),
    ))
}

pub async fn confirm(
    owner: PlatformUser,
    Extension(login): Extension<Arc<dyn LoginUseCase>>,
    request: Result<Json<ConfirmRequest>, JsonRejection>,
) -> Result<StatusCode, AuthError> {
    let Json(request) = request.map_err(|_| invalid_confirmation())?;
    if request.current_password.trim().is_empty()
        || request.code.len() != 6
        || !request.code.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid_confirmation());
    }
    login
        .confirm_totp(
            owner.user_id,
            request.enrollment_id,
            &request.current_password,
            &request.code,
        )
        .await
        .map_err(|error| enrollment_error(&error))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel(
    owner: PlatformUser,
    Extension(login): Extension<Arc<dyn LoginUseCase>>,
    Path(enrollment_id): Path<Uuid>,
) -> Result<StatusCode, AuthError> {
    login
        .cancel_totp(owner.user_id, enrollment_id)
        .await
        .map_err(|error| enrollment_error(&error))?;
    Ok(StatusCode::NO_CONTENT)
}

fn invalid_confirmation() -> AuthError {
    protocol_error(
        "totp_enrollment",
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_totp_confirmation",
        "Invalid authenticator confirmation",
    )
}

fn enrollment_error(error: &TotpError) -> AuthError {
    let (status, code, message) = match error {
        TotpError::InvalidProof | TotpError::Required => (
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Invalid password or authenticator code",
        ),
        TotpError::NotFound => (
            StatusCode::NOT_FOUND,
            "enrollment_not_found",
            "Enrollment not found",
        ),
        TotpError::Conflict => (
            StatusCode::CONFLICT,
            "enrollment_not_available",
            "Initial enrollment is not available for this account",
        ),
        TotpError::Throttled => (
            StatusCode::TOO_MANY_REQUESTS,
            "totp_throttled",
            "Too many authenticator attempts",
        ),
        TotpError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "totp_unavailable",
            "Authenticator service unavailable",
        ),
    };
    protocol_error("totp_enrollment", status, code, message)
}

pub fn protocol_error(operation: &str, status: StatusCode, code: &str, message: &str) -> AuthError {
    AuthError::OAuth {
        operation: operation.to_owned(),
        error_code: code.to_owned(),
        message: message.to_owned(),
        status,
    }
}
