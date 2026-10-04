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

#[path = "support/registry.rs"]
mod registry;

use axum::http::StatusCode;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ext_authz::{check_router, AuthzState, ExtAuthzConfig};
use jwks_fixtures::JwksFixtures;
use registry::platform_jwks as test_rs256_jwks_json;
use rustycog::testing::http::jwt::{
    create_rs256_jwt_token, create_rs256_jwt_token_with_options, Rs256TokenOptions,
    TEST_PLATFORM_ISSUER, TEST_RS256_PRIVATE_PEM,
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

// Sign altered claims with the same fixed, nonsecret test key and valid kid.
// Unlike an unknown-kid denial, these cases reach the signature/claim verifier.
fn signed_variant(token: &str, change: impl FnOnce(&mut serde_json::Value)) -> String {
    let parts: Vec<_> = token.split('.').collect();
    let mut claims: serde_json::Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[1])
            .expect("fixture claims encoding"),
    )
    .expect("fixture claims JSON");
    change(&mut claims);
    let header = jsonwebtoken::decode_header(token).expect("fixture header");
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RS256_PRIVATE_PEM.as_bytes())
        .expect("fixed test signing key");
    jsonwebtoken::encode(&header, &claims, &key).expect("sign variant")
}

#[tokio::test]
#[serial]
async fn standin_rejects_expired_and_future_claims_with_valid_kid() {
    let token = create_rs256_jwt_token(Uuid::new_v4());
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
    // Positive control: a malformed/unauthorized JWKS must not make negatives green.
    server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::OK);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let variants = [
        signed_variant(&token, |claims| claims["exp"] = (now - 3600).into()),
        signed_variant(&token, |claims| claims["nbf"] = (now + 3600).into()),
        signed_variant(&token, |claims| claims["iat"] = (now + 3600).into()),
    ];
    for invalid in variants {
        let response = server
            .post("/")
            .add_header("authorization", format!("Bearer {invalid}"))
            .await;
        response.assert_status(StatusCode::FORBIDDEN);
        assert!(response.maybe_header("x-principal-iss").is_none());
        assert!(response.maybe_header("x-principal-sub").is_none());
        assert!(response.maybe_header("x-principal-org").is_none());
    }
}

#[tokio::test]
#[serial]
async fn standin_rejects_bad_signature_with_valid_kid() {
    let token = create_rs256_jwt_token(Uuid::new_v4());
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
    server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::OK);
    let mut parts: Vec<_> = token.split('.').map(str::to_owned).collect();
    let mut signature = URL_SAFE_NO_PAD
        .decode(&parts[2])
        .expect("signature encoding");
    signature[0] ^= 1;
    parts[2] = URL_SAFE_NO_PAD.encode(signature);
    let response = server
        .post("/")
        .add_header("authorization", format!("Bearer {}", parts.join(".")))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
    assert!(response.maybe_header("x-principal-iss").is_none());
    assert!(response.maybe_header("x-principal-sub").is_none());
    assert!(response.maybe_header("x-principal-org").is_none());
}

#[tokio::test]
#[serial]
async fn standin_rejects_wrong_token_type_with_valid_kid() {
    let user = Uuid::new_v4();
    let token = create_rs256_jwt_token(user);
    let (server, _jwks) = server_with_jwks(&test_rs256_jwks_json()).await;
    server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::OK);
    for typ in [None, Some("JWT"), Some("aiforall-refresh+jwt")] {
        let invalid = create_rs256_jwt_token_with_options(
            user,
            Rs256TokenOptions {
                typ: Some(typ),
                ..Rs256TokenOptions::default()
            },
        );
        let response = server
            .post("/")
            .add_header("authorization", format!("Bearer {invalid}"))
            .await;
        response.assert_status(StatusCode::FORBIDDEN);
        assert!(response.maybe_header("x-principal-iss").is_none());
        assert!(response.maybe_header("x-principal-sub").is_none());
        assert!(response.maybe_header("x-principal-org").is_none());
    }
}

#[tokio::test]
#[serial]
async fn real_organization_metadata_accepts_owner_issuer_but_not_platform_substitution() {
    use iam_domain::entity::signing_key::SigningKeyStatus;
    let organization = Uuid::new_v4();
    let key = registry::registry_key(SigningKeyStatus::Active, Some(organization));
    let (server, _jwks) = server_with_jwks(&registry::serialize(&[key.clone()])).await;
    let token = create_rs256_jwt_token(Uuid::new_v4());
    let legitimate = signed_variant(&token, |claims| {
        claims["iss"] = key.issuer.clone().into();
        claims["org"] = organization.to_string().into();
    });
    // Positive control proves the publisher key is actually trusted as an org principal.
    server
        .post("/")
        .add_header("authorization", format!("Bearer {legitimate}"))
        .await
        .assert_status(StatusCode::OK);
    // Same signing key/kid/sub, but claims pretend it is the platform issuer.
    let response = server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
    assert!(response.maybe_header("x-principal-sub").is_none());
    for invalid in [
        signed_variant(&legitimate, |claims| {
            claims["org"] = Uuid::new_v4().to_string().into()
        }),
        signed_variant(&legitimate, |claims| {
            claims.as_object_mut().expect("claims object").remove("org");
        }),
    ] {
        let response = server
            .post("/")
            .add_header("authorization", format!("Bearer {invalid}"))
            .await;
        response.assert_status(StatusCode::FORBIDDEN);
        assert!(response.maybe_header("x-principal-sub").is_none());
    }
}

#[tokio::test]
#[serial]
async fn real_pending_publisher_key_cannot_authenticate() {
    use iam_domain::entity::signing_key::SigningKeyStatus;
    let key = registry::registry_key(SigningKeyStatus::Pending, None);
    let published = registry::serialize(&[key]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&published).expect("JWKS")["keys"][0]["status"],
        "pending"
    );
    let (server, _jwks) = server_with_jwks(&published).await;
    let token = create_rs256_jwt_token(Uuid::new_v4());
    server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn real_platform_metadata_cannot_be_promoted_to_organization_by_a_claim() {
    let (server, _jwks) = server_with_jwks(&registry::platform_jwks()).await;
    let user = Uuid::new_v4();
    let token = create_rs256_jwt_token(user);
    server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::OK);
    let organization = Uuid::new_v4();
    let with_org = signed_variant(&token, |claims| {
        claims["org"] = organization.to_string().into()
    });
    // An extra claim cannot change the authenticated platform (iss, sub).
    let response = server
        .post("/")
        .add_header("authorization", format!("Bearer {with_org}"))
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
    let invalid = signed_variant(&token, |claims| {
        claims["iss"] = format!("https://issuer.example/org/{organization}").into();
        claims["org"] = organization.to_string().into()
    });
    let rejected = server
        .post("/")
        .add_header("authorization", format!("Bearer {invalid}"))
        .await;
    rejected.assert_status(StatusCode::FORBIDDEN);
    assert!(rejected.maybe_header("x-principal-iss").is_none());
    assert!(rejected.maybe_header("x-principal-sub").is_none());
    assert!(rejected.maybe_header("x-principal-org").is_none());
}

#[tokio::test]
#[serial]
async fn real_retiring_metadata_still_verifies_existing_tokens() {
    use iam_domain::entity::signing_key::SigningKeyStatus;
    let key = registry::registry_key(SigningKeyStatus::Retiring, None);
    let (server, _jwks) = server_with_jwks(&registry::serialize(&[key])).await;
    let token = create_rs256_jwt_token(Uuid::new_v4());
    server
        .post("/")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::OK);
}
