use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{info_span, Instrument};
use uuid::Uuid;

use crate::middleware_auth::AuthUser;

/// Header name for request ID
pub const X_REQUEST_ID: &str = "x-request-id";
/// Header name for correlation ID
pub const X_CORRELATION_ID: &str = "x-correlation-id";

/// Tracing middleware that adds span information for request tracking
pub async fn tracing_middleware(mut request: Request, next: Next) -> Response {
    let start_time = Instant::now();

    // Extract or generate correlation ID
    let correlation_id = match request
        .headers()
        .get(X_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
    {
        Some(id) => id.to_string(),
        None => {
            let id = Uuid::new_v4().to_string();
            // Insert the generated correlation ID into the request headers
            request.headers_mut().insert(
                X_CORRELATION_ID,
                id.parse()
                    .expect("Generated UUID should be valid header value"),
            );
            id
        }
    };

    // Extract request ID if present
    let request_id = request
        .headers()
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Extract user ID from request extensions if authenticated
    let user_id = request
        .extensions()
        .get::<AuthUser>()
        .map(|auth_user| auth_user.user_id.clone());

    // Get the request method and URI for logging
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path();

    // Create a span with all available information
    let span = if let Some(req_id) = &request_id {
        if let Some(uid) = &user_id {
            info_span!(
                "http_request",
                %method,
                %path,
                %correlation_id,
                request_id = %req_id,
                user_id = %uid,
                response_time_ms = tracing::field::Empty,
                status_code = tracing::field::Empty,
            )
        } else {
            info_span!(
                "http_request",
                %method,
                %path,
                %correlation_id,
                request_id = %req_id,
                response_time_ms = tracing::field::Empty,
                status_code = tracing::field::Empty,
            )
        }
    } else {
        if let Some(uid) = &user_id {
            info_span!(
                "http_request",
                %method,
                %path,
                %correlation_id,
                user_id = %uid,
                response_time_ms = tracing::field::Empty,
                status_code = tracing::field::Empty,
            )
        } else {
            info_span!(
                "http_request",
                %method,
                %path,
                %correlation_id,
                response_time_ms = tracing::field::Empty,
                status_code = tracing::field::Empty,
            )
        }
    };

    // Process the request within the span
    let response = next.run(request).instrument(span.clone()).await;

    // Calculate response time and record final span data
    let duration = start_time.elapsed();
    let status_code = response.status().as_u16();

    span.record("response_time_ms", duration.as_millis());
    span.record("status_code", status_code);

    response
}

/// Extract correlation ID from request headers
pub fn get_correlation_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(X_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract request ID from request headers
pub fn get_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}
