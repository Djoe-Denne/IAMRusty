use axum::{
    Router, 
    routing::{get},
    extract::{Path, Query, State},
    response::{Redirect, IntoResponse},
    Json,
};
use application::service::AuthService;
use application::dto::ProviderTokenResponseDto;
use serde::{Deserialize, Serialize};
use crate::error::ApiError;
use crate::extractor::JwtAuth;

/// Routes for the auth module
pub fn routes<U, T>(auth_service: AuthService<U, T>) -> Router
where
    U: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    Router::new()
        .route("/:provider/start", get(start_auth))
        .route("/:provider/callback", get(callback))
        .route("/internal/:provider/token", get(get_provider_token))
        .with_state(auth_service)
}

/// Query parameters for callback
#[derive(Debug, Deserialize)]
struct CallbackQuery {
    /// Authorization code
    code: String,
}

/// Start OAuth2 authentication
async fn start_auth(
    Path(provider): Path<String>,
    State(auth_service): State<AuthService<impl Send + Sync, impl Send + Sync>>,
) -> Result<Redirect, ApiError> {
    let url = auth_service
        .generate_authorize_url(&provider)
        .map_err(ApiError::Application)?;
    
    Ok(Redirect::to(&url))
}

/// Handle OAuth2 callback
async fn callback(
    Path(provider): Path<String>,
    Query(query): Query<CallbackQuery>,
    State(auth_service): State<AuthService<impl Send + Sync, impl Send + Sync>>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_response = auth_service
        .process_callback(&provider, &query.code)
        .await
        .map_err(ApiError::Application)?;
    
    Ok(Json(auth_response))
}

/// Get a provider token for internal use
async fn get_provider_token(
    Path(provider): Path<String>,
    auth: JwtAuth,
    State(auth_service): State<AuthService<impl Send + Sync, impl Send + Sync>>,
) -> Result<impl IntoResponse, ApiError> {
    let tokens = auth_service
        .get_provider_token(&auth.0.sub, &provider)
        .await
        .map_err(ApiError::Application)?;
    
    Ok(Json(ProviderTokenResponseDto::from(tokens)))
} 