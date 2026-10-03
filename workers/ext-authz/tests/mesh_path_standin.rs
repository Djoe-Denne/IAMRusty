//! STAND-IN du Check Envoy HTTP ; n'affirme pas qu'Envoy a tourné.
//!
//! Preuve cargo du contrat strip + recreate (iss/sub) / deny spoof, sans Docker
//! ni processus Envoy. Câblage runtime = `docker compose --profile mesh` et
//! `ops/deploy/apps/overlays/kind-mesh/` (opt-in).
//!
//! Smoke Bearer défaut des services : inchangé — `runtime/monolith/prove-e2e-curl.ps1`
//! et `*/setup/src/app.rs` non modifiés ; JWT rustycog in-process reste le défaut
//! (ADR-0308 Partial, pas Implemented).

#![allow(missing_docs)]

#[path = "../src/jwks_fixtures.rs"]
mod jwks_fixtures;

use axum::http::StatusCode;
use ext_authz::{check_router, AuthzState, ExtAuthzConfig};
use jwks_fixtures::JwksFixtures;
use rustycog::testing::http::jwt::{
    create_rs256_jwt_token, test_rs256_jwks_json, TEST_PLATFORM_ISSUER,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

async fn server_with_jwks(jwks: &str) -> (axum_test::TestServer, JwksFixtures) {
    let fixture = JwksFixtures::service().await;
    fixture.mock_jwks_ok(jwks).await;
    let config = ExtAuthzConfig {
        jwks_url: format!("{}/.well-known/jwks.json", fixture.base_url()),
        audience: rustycog::testing::http::jwt::TEST_JWT_AUDIENCE.to_string(),
        poll_interval: Duration::from_secs(3600),
        negative_cache_ttl: Duration::from_secs(30),
    };
    let state = Arc::new(AuthzState::new(config).expect("state"));
    let server = axum_test::TestServer::new(check_router(state)).expect("server");
    (server, fixture)
}

#[tokio::test]
#[serial]
async fn standin_strips_client_x_principal_and_recreates_iss_sub() {
    let user = Uuid::new_v4();
    let token = create_rs256_jwt_token(user);
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;

    let response = server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .add_header("x-principal-iss", "https://spoofed.example")
        .add_header("x-principal-sub", "00000000-0000-0000-0000-000000000000")
        .add_header("x-principal-org", "attacker-org")
        .await;

    response.assert_status(StatusCode::OK);
    assert_eq!(
        response.header("x-principal-iss").to_str().expect("iss"),
        TEST_PLATFORM_ISSUER
    );
    assert_eq!(
        response.header("x-principal-sub").to_str().expect("sub"),
        user.to_string()
    );
    assert!(response.maybe_header("x-principal-org").is_none());
}

#[tokio::test]
#[serial]
async fn standin_rejects_spoofed_x_principal_without_bearer() {
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
    let response = server
        .post("/")
        .add_header("x-principal-iss", TEST_PLATFORM_ISSUER)
        .add_header("x-principal-sub", Uuid::new_v4().to_string())
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
    assert!(response.maybe_header("x-principal-iss").is_none());
    assert!(response.maybe_header("x-principal-sub").is_none());
}

#[tokio::test]
#[serial]
async fn standin_allows_bearer_without_x_principal() {
    let user = Uuid::new_v4();
    let token = create_rs256_jwt_token(user);
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;

    let response = server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    assert_eq!(
        response.header("x-principal-iss").to_str().expect("iss"),
        TEST_PLATFORM_ISSUER
    );
    assert_eq!(
        response.header("x-principal-sub").to_str().expect("sub"),
        user.to_string()
    );
}

#[tokio::test]
#[serial]
async fn standin_envoy_body_spoofed_x_principal_is_ignored() {
    let user = Uuid::new_v4();
    let token = create_rs256_jwt_token(user);
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;

    // Envoy Check body shape: spoofed x-principal-* in attributes must be ignored;
    // Bearer arrives on HTTP headers (as Envoy authorization_request forwards it).
    let body = serde_json::json!({
        "attributes": {
            "request": {
                "http": {
                    "headers": {
                        "x-principal-iss": "https://body-spoof.example",
                        "x-principal-sub": "11111111-1111-1111-1111-111111111111",
                        "x-principal-org": "body-attacker",
                        "authorization": format!("Bearer {token}")
                    }
                }
            }
        }
    });

    let response = server
        .post("/check")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&body)
        .await;

    response.assert_status(StatusCode::OK);
    assert_eq!(
        response.header("x-principal-iss").to_str().expect("iss"),
        TEST_PLATFORM_ISSUER
    );
    assert_eq!(
        response.header("x-principal-sub").to_str().expect("sub"),
        user.to_string()
    );
    assert!(response.maybe_header("x-principal-org").is_none());
}
