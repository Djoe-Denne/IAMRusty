use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::debug;

use application::command::{user::ValidateTokenCommand, CommandContext, CommandError};
use std::collections::HashMap;
use uuid::Uuid;

/// Authenticated user information extracted from middleware
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user_id = parts
            .extensions
            .get::<Uuid>()
            .copied()
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser { user_id })
    }
}

/// Extract JWT token from the Authorization header
fn extract_token(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}

/// Authentication middleware
pub async fn auth(
    State(state): State<crate::AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get the Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract the token
    let token = extract_token(auth_header).ok_or(StatusCode::UNAUTHORIZED)?;

    // Create command context for token validation
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let context = CommandContext {
        execution_id: Uuid::new_v4(),
        user_id: None,
        request_id: request_id.clone(),
        metadata: HashMap::new(),
    };

    debug!(
        "Try to validate token for query {}",
        request_id.unwrap_or_default()
    );

    // Validate the token using command service
    let command = ValidateTokenCommand::new(token.to_string());
    let user_id = state
        .command_service
        .execute(command, context)
        .await
        .map_err(|e| match e {
            CommandError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            CommandError::Business { .. } => StatusCode::UNAUTHORIZED,
            CommandError::Validation { .. } => StatusCode::UNAUTHORIZED,
            CommandError::Infrastructure { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            CommandError::Timeout { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            CommandError::RetryExhausted { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // Add the user ID to the request extensions
    let mut req = req;
    req.extensions_mut().insert(user_id);

    // Continue with the request
    Ok(next.run(req).await)
}
