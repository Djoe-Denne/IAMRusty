//! A single B TCP witness using SDK ownership, not Application/binary startup C.

use super::r_fixture::{must, Reply};
use axum::Router;
use rustycog::{
    config::ServerConfig,
    http::{start_router_owned, HttpShutdownBudget},
};
use std::{future::Future, time::Duration};

/// Preserve the SDK's 5s grace/6s deadline, including when assertions panic.
pub async fn run<F, Fut>(router: Router, check: F)
where
    F: FnOnce(String) -> Fut + Send,
    Fut: Future<Output = ()> + Send + 'static,
{
    let config = ServerConfig {
        host: "127.0.0.1".into(),
        port: 0,
        tls_enabled: false,
        tls_port: 0,
        ..ServerConfig::default()
    };
    let mut server = must(
        start_router_owned(router, config).await,
        "SDK-owned listener bind/start",
    );
    let addresses = server.local_addrs();
    if addresses.len() != 1 || addresses[0].port() == 0 {
        let now = tokio::time::Instant::now();
        server.request_shutdown(must(
            HttpShutdownBudget::new(now + Duration::from_secs(5), now + Duration::from_secs(6)),
            "SDK shutdown budget",
        ));
        let _ = server.join().await;
        panic!("SDK listener address preflight failed, not a routing RED");
    }
    let mut assertions = tokio::spawn(check(format!("http://{}", addresses[0])));
    let result = tokio::time::timeout(Duration::from_secs(10), &mut assertions).await;
    if result.is_err() {
        assertions.abort();
        let _ = assertions.await;
    }
    let now = tokio::time::Instant::now();
    server.request_shutdown(must(
        HttpShutdownBudget::new(now + Duration::from_secs(5), now + Duration::from_secs(6)),
        "SDK shutdown budget",
    ));
    let report = must(server.join().await, "SDK-owned listener complete join");
    assert!(
        !report.forced,
        "G-6: normal/failed-assertion witness must join without forced shutdown"
    );
    result
        .unwrap_or_else(|_| panic!("owned routing test watchdog fired, not a PASS"))
        .unwrap_or_else(|error| {
            if error.is_panic() {
                std::panic::resume_unwind(error.into_panic());
            }
            panic!("owned routing assertion task cancelled");
        });
}

/// Bound every incoming-service response read in this test as well.
pub async fn reply(mut response: reqwest::Response) -> Reply {
    let status = response.status();
    let mut body = Vec::new();
    while let Some(chunk) = must(response.chunk().await, "bounded TCP response chunk") {
        assert!(
            chunk.len() <= 8192 - body.len(),
            "routing fixture response exceeds allowance"
        );
        body.extend_from_slice(&chunk);
    }
    Reply { status, body }
}
