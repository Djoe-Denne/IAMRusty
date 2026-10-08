//! S5: real accepted TCP source, no manually injected ConnectInfo/peer certificate.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;
use iam_configuration::security::AuthRateLimitConfig;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn changing_untrusted_xff_cannot_change_the_actual_transport_source_budget() {
    let limits = AuthRateLimitConfig {
        source_limit: 2,
        account_limit: 1000,
        ..AuthRateLimitConfig::default()
    };
    let (fixture, base, client) = Box::pin(common::setup_test_server_with_rate_limits(limits))
        .await
        .expect("real listener with limiter explicitly enabled");
    fixture_cleanup::run(&fixture, async {
    for (index, source) in ["198.51.100.1", "203.0.113.2", "192.0.2.3"]
        .into_iter()
        .enumerate()
    {
        let response = client.post(format!("{base}/api/auth/login"))
            .header("x-forwarded-for", source)
            .header("x-principal-sub", uuid::Uuid::new_v4().to_string())
            .json(&json!({"email": format!("source-budget-{index}@example.com"), "password": "SentinelABC-WrongPassword1a!"}))
            .send().await.expect("real TCP login request");
        assert_eq!(
            response.status().as_u16(),
            if index < 2 { 401 } else { 429 },
            "OS source must remain the same; arbitrary XFF/principal fields confer no proxy trust"
        );
        let body = response.text().await.expect("public error body");
        assert!(!body.contains("SentinelABC"));
    }
    }).await;
}
