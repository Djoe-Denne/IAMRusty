//! HTTP routes

mod auth;
mod user;
mod jwks;

use axum::{Router, routing::get};
use application::service::{AuthService, TokenService};

/// Build the API router
pub fn api_router<U, T>(
    auth_service: AuthService<U, T>,
    token_service: TokenService,
) -> Router 
where 
    U: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let auth_routes = auth::routes(auth_service.clone());
    let user_routes = user::routes(auth_service);
    let jwks_routes = jwks::routes(token_service.clone());
    
    Router::new()
        .nest("/auth", auth_routes)
        .nest("/", user_routes)
        .nest("/.well-known", jwks_routes)
        .with_state(token_service)
} 