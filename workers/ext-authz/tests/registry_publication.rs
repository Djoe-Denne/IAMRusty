//! HTTP collaborator + real IAM publisher serialization. No Envoy/runtime proof.
#[path = "../src/jwks_fixtures.rs"]
mod jwks_fixtures;
#[path = "support/registry.rs"]
mod registry;

use ext_authz::JwksCache;
use iam_domain::entity::signing_key::SigningKeyStatus;
use jwks_fixtures::JwksFixtures;
use registry::{registry_key, serialize};
use rustycog::testing::http::jwt::TEST_RS256_KID;
use serial_test::serial;
use std::time::Duration;

#[tokio::test]
#[serial]
async fn actual_publisher_last_key_revoke_is_authoritative_empty() {
    let fixture = JwksFixtures::service().await;
    let mut key = registry_key(SigningKeyStatus::Active, None);
    fixture.mock_jwks_ok(&serialize(&[key.clone()])).await;
    let cache = JwksCache::new(
        format!("{}/.well-known/jwks.json", fixture.base_url()),
        Duration::ZERO,
        Duration::ZERO,
    )
    .expect("cache");
    cache
        .document_for_kid(TEST_RS256_KID)
        .await
        .expect("active publisher key");
    key.status = SigningKeyStatus::Revoked;
    let empty = serialize(&[key]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&empty).expect("JWKS")["keys"],
        serde_json::json!([])
    );
    fixture.reset().await;
    fixture.mock_jwks_ok(&empty).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    cache.poll_if_due().await;
    assert!(
        cache.document_for_kid(TEST_RS256_KID).await.is_err(),
        "empty must revoke the known kid, not resurrect bootstrap"
    );
    fixture.reset().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    cache.poll_if_due().await;
    assert!(
        cache.document_for_kid(TEST_RS256_KID).await.is_err(),
        "outage after empty must not resurrect the prior key"
    );
}

#[tokio::test]
#[serial]
async fn outage_bounds_all_known_kids_and_recovery_replaces_snapshot() {
    let fixture = JwksFixtures::service().await;
    let first = registry_key(SigningKeyStatus::Active, None);
    let mut second = first.clone();
    second.kid = "second-known-kid".into();
    fixture
        .mock_jwks_ok(&serialize(&[first.clone(), second.clone()]))
        .await;
    let cache = JwksCache::new(
        format!("{}/.well-known/jwks.json", fixture.base_url()),
        Duration::ZERO,
        Duration::ZERO,
    )
    .expect("cache");
    cache.document_for_kid(&first.kid).await.expect("first key");
    cache
        .document_for_kid(&second.kid)
        .await
        .expect("second key");
    fixture.reset().await; // Unmatched HTTP request => outage, NOT an empty JSON snapshot.
    cache.poll_if_due().await;
    tokio::time::sleep(Duration::from_secs(59)).await;
    for kid in [&first.kid, &second.kid] {
        cache
            .document_for_kid(kid)
            .await
            .expect("fresh snapshot within 60-second budget");
    }
    // Real monotonic time, no claim that tokio's paused clock advances std::Instant.
    tokio::time::sleep(Duration::from_secs(2)).await;
    for kid in [&first.kid, &second.kid] {
        assert!(
            cache.document_for_kid(kid).await.is_err(),
            "every known key must fail after the budget"
        );
    }
    let mut recovered = first;
    recovered.kid = "recovered-key".into();
    fixture.mock_jwks_ok(&serialize(&[recovered.clone()])).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    cache.poll_if_due().await;
    cache
        .document_for_kid(&recovered.kid)
        .await
        .expect("new snapshot recovered");
    assert!(
        cache.document_for_kid(&second.kid).await.is_err(),
        "old kid must stay absent after recovery"
    );
}
