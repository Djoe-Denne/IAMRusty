use std::sync::Arc;

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use iam_application::usecase::profile_email_verification::{
    ProfileEmailVerification, ProfileVerificationError, SavedAddress,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::PlatformUser;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IssueRequest {
    address_id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmRequest {
    token: String,
}

pub(crate) async fn list(
    user: PlatformUser,
    Extension(verification): Extension<Arc<ProfileEmailVerification>>,
) -> Result<Json<Vec<SavedAddress>>, StatusCode> {
    verification
        .list(user.user_id)
        .await
        .map(Json)
        .map_err(status)
}

pub(crate) async fn issue(
    user: PlatformUser,
    Extension(verification): Extension<Arc<ProfileEmailVerification>>,
    request: Result<Json<IssueRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = request else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let result = verification.issue(user.user_id, request.address_id).await;
    match result {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(ProfileVerificationError::Cooldown) => (
            StatusCode::TOO_MANY_REQUESTS,
            [(axum::http::header::RETRY_AFTER, "60")],
        )
            .into_response(),
        Err(error) => status(error).into_response(),
    }
}

pub(crate) async fn confirm(
    user: PlatformUser,
    Extension(verification): Extension<Arc<ProfileEmailVerification>>,
    request: Result<Json<ConfirmRequest>, JsonRejection>,
) -> StatusCode {
    let Ok(Json(request)) = request else {
        return StatusCode::BAD_REQUEST;
    };
    verification
        .confirm(user.user_id, &request.token)
        .await
        .map_or_else(status, |()| StatusCode::NO_CONTENT)
}

fn status(error: ProfileVerificationError) -> StatusCode {
    match error {
        ProfileVerificationError::NotEligible => StatusCode::NOT_FOUND,
        ProfileVerificationError::Cooldown => StatusCode::TOO_MANY_REQUESTS,
        ProfileVerificationError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        ProfileVerificationError::Internal => {
            tracing::warn!("Profile email verification operation failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
