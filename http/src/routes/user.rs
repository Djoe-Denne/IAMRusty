use axum::{
    Router, 
    routing::{get},
    extract::State,
    response::IntoResponse,
    Json,
};
use application::service::AuthService;
use crate::error::ApiError;
use crate::extractor::JwtAuth;

/// Routes for the user module
pub fn routes<U, T>(auth_service: AuthService<U, T>) -> Router
where
    U: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    Router::new()
        .route("/me", get(get_current_user))
        .with_state(auth_service)
}

/// Get the current user
async fn get_current_user(
    auth: JwtAuth,
    State(auth_service): State<AuthService<impl Send + Sync, impl Send + Sync>>,
) -> Result<impl IntoResponse, ApiError> {
    let user = auth_service
        .find_user_by_id(&auth.0.sub)
        .await
        .map_err(ApiError::Application)?;
    
    Ok(Json(user))
} 