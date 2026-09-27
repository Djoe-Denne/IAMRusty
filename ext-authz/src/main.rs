//! ext_authz HTTP Check binary (ADR-0308). Overlay Envoy Kind stays commented.
#![allow(missing_docs)]

use ext_authz::{check_router, AuthzState, ExtAuthzConfig};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let config = ExtAuthzConfig::from_env().expect("ext-authz config");
    let poll_interval = config.poll_interval;
    let state = std::sync::Arc::new(AuthzState::new(config).expect("ext-authz state"));
    let poller = state.clone();
    tokio::spawn(async move {
        let tick = if poll_interval.is_zero() {
            Duration::from_secs(1)
        } else {
            poll_interval.min(Duration::from_secs(30))
        };
        loop {
            poller.poll_jwks_if_due().await;
            tokio::time::sleep(tick).await;
        }
    });
    let app = check_router(state);
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8090);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
