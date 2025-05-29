use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};

use application::command::{CommandContext, CommandError};
use uuid::Uuid;
use std::collections::HashMap;

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
    let context = CommandContext {
        execution_id: Uuid::new_v4(),
        user_id: None,
        request_id: req.headers()
            .get("x-request-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
        metadata: HashMap::new(),
    };

    // Validate the token using command service
    let user_id = state
        .command_service
        .validate_token(token.to_string(), context)
        .await
        .map_err(|e| match e {
            CommandError::Authentication(_) => StatusCode::UNAUTHORIZED,
            CommandError::Business(_) => StatusCode::UNAUTHORIZED,
            CommandError::Validation(_) => StatusCode::UNAUTHORIZED,
            CommandError::Infrastructure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CommandError::Timeout => StatusCode::INTERNAL_SERVER_ERROR,
            CommandError::RetryExhausted(_) => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // Add the user ID to the request extensions
    let mut req = req;
    req.extensions_mut().insert(user_id);

    // Continue with the request
    Ok(next.run(req).await)
} 