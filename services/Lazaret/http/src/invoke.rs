//! `POST /invoke` — P0 [`InvokeRequest`] bound to the current Lazaret session + live grants.

use std::sync::Arc;

use apparatus_contracts::{InvokeRequest, InvokeResponse};
use axum::extract::{Extension, Query};
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use axum::Json;
use lazaret_application::InvokeService;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::InvokeHttpError;

#[derive(Debug, Deserialize)]
pub struct InvokeQuery {
    project_id: Uuid,
}

/// `POST /invoke` under the service prefix.
pub async fn invoke(
    Extension(invoke): Extension<Arc<InvokeService>>,
    Query(query): Query<InvokeQuery>,
    headers: HeaderMap,
    Json(request): Json<InvokeRequest>,
) -> Result<Json<InvokeResponse>, InvokeHttpError> {
    let token = bearer_token(&headers).ok_or(InvokeHttpError::Unauthorized)?;
    let response = invoke.invoke(token, query.project_id, request).await?;
    Ok(Json(response))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
}
