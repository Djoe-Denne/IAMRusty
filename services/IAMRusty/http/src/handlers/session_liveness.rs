//! Internal authoritative session-liveness endpoint.

use axum::{
    extract::{rejection::JsonRejection, Extension, FromRequestParts},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, StatusCode},
    Json,
};
use rustycog::http::{
    SessionLivenessChecker, SessionLivenessDecision as LivenessDecision, SessionLivenessError,
    SessionLivenessQuery,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::platform_user::PlatformIssuer;
use crate::rate_limit::require_internal_service_token;

pub struct InternalServiceToken;

impl<S: Send + Sync> FromRequestParts<S> for InternalServiceToken {
    type Rejection = StatusCode;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(require_internal_service_token(&parts.headers).map(|()| Self))
    }
}

#[derive(Debug, Deserialize)]
pub struct SessionLivenessRequest {
    issuer: String,
    subject: Uuid,
    #[serde(default, deserialize_with = "deserialize_optional_epoch")]
    auth_epoch: Option<String>,
}

fn deserialize_optional_epoch<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(value) => Ok(Some(value)),
        _ => Err(serde::de::Error::custom(
            "auth_epoch must be a canonical decimal string",
        )),
    }
}

#[derive(Debug, Serialize)]
pub struct SessionLivenessResponse {
    decision: SessionLivenessDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_epoch: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLivenessDecision {
    Live,
    Revoked,
    NotApplicable,
}

/// Return an authoritative liveness decision for a verified platform credential.
///
/// # Errors
///
/// Returns 403 for an invalid workload token, 400 for a malformed epoch, and
/// 503 when the authoritative decision cannot be obtained.
pub async fn session_liveness(
    _token: InternalServiceToken,
    Extension(platform_issuer): Extension<PlatformIssuer>,
    Extension(checker): Extension<Arc<dyn SessionLivenessChecker>>,
    request: Result<Json<SessionLivenessRequest>, JsonRejection>,
) -> Result<(HeaderMap, Json<SessionLivenessResponse>), StatusCode> {
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    let auth_epoch = request
        .auth_epoch
        .as_deref()
        .map(parse_epoch)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let platform_query = platform_issuer.as_str() == request.issuer;
    let outcome = checker
        .check(&SessionLivenessQuery {
            issuer: request.issuer,
            subject: request.subject,
            auth_epoch,
        })
        .await
        .map_err(|error| match error {
            SessionLivenessError::Revoked => StatusCode::UNAUTHORIZED,
            SessionLivenessError::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        })?;
    if platform_query && outcome.decision == LivenessDecision::NotApplicable {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let decision = match outcome.decision {
        LivenessDecision::Live => SessionLivenessDecision::Live,
        LivenessDecision::Revoked => SessionLivenessDecision::Revoked,
        LivenessDecision::NotApplicable => SessionLivenessDecision::NotApplicable,
    };
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((
        headers,
        Json(SessionLivenessResponse {
            decision,
            current_epoch: outcome.current_epoch.map(|epoch| epoch.to_string()),
        }),
    ))
}

fn parse_epoch(value: &str) -> Result<u64, &'static str> {
    let canonical = !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0'));
    if !canonical {
        return Err("invalid epoch");
    }
    value.parse::<u64>().map_err(|_| "invalid epoch")
}
