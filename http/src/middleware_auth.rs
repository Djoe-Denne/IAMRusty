use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};
use crate::AppState;
use application::usecase::user::UserError;

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
    State(state): State<AppState>,
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

    // Validate the token
    let user_id = state
        .user_usecase
        .validate_token(token)
        .await
        .map_err(|e| match e {
            UserError::InvalidToken => StatusCode::UNAUTHORIZED,
            UserError::TokenExpired => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // Add the user ID to the request extensions
    let mut req = req;
    req.extensions_mut().insert(user_id);

    // Continue with the request
    Ok(next.run(req).await)
} 