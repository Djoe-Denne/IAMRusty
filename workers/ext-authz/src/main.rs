//! `ext_authz` HTTP Check binary (ADR-0308). Overlay Envoy `Kind` stays commented.
#![allow(missing_docs)]

mod tls_server;

use ext_authz::{check_router, AuthzState, ExtAuthzConfig};
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let config = match ExtAuthzConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("ext-authz: {err}");
            std::process::exit(1);
        }
    };
    let poll_interval = config.poll_interval;
    let state = match AuthzState::new(config) {
        Ok(state) => std::sync::Arc::new(state),
        Err(err) => {
            eprintln!("ext-authz: {err}");
            std::process::exit(1);
        }
    };
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
    if let Err(err) = tls_server::serve(app, port).await {
        eprintln!("ext-authz: {err}");
        std::process::exit(1);
    }
}
