use axum::{Router, http::Method};
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tower_http::catch_panic::CatchPanicLayer;
use std::net::SocketAddr;
use tracing::Level;

use crate::routes::api_router;
use application::service::{AuthService, TokenService};

/// Initialize and start the HTTP server
pub async fn serve<U, T>(
    auth_service: AuthService<U, T>,
    token_service: TokenService,
    host: &str,
    port: u16,
) -> Result<(), hyper::Error> 
where
    U: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Build the router
    let app = Router::new()
        // Add the API routes
        .merge(api_router(auth_service, token_service))
        // Add middleware
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new().level(Level::INFO)
                )
                .on_response(
                    DefaultOnResponse::new().level(Level::INFO)
                )
        )
        .layer(CatchPanicLayer::new());

    // Create the server address
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid server address");

    // Start the server
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
} 