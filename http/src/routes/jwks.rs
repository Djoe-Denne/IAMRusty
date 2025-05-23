use axum::{
    Router, 
    routing::{get},
    extract::State,
    response::IntoResponse,
    Json,
};
use application::service::TokenService;

/// Routes for the JWKS module
pub fn routes(token_service: TokenService) -> Router {
    Router::new()
        .route("/jwks.json", get(get_jwks))
        .with_state(token_service)
}

/// Get the JSON Web Key Set
async fn get_jwks(
    State(token_service): State<TokenService>,
) -> impl IntoResponse {
    let jwks = token_service.jwks();
    Json(jwks)
} 