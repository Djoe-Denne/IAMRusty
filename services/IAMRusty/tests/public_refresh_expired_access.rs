//! Public refresh with a genuinely expired, correctly signed published-key access JWT.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../../../workers/ext-authz/tests/support/registry.rs"]
mod registry;
mod utils;

use iam_domain::entity::signing_key::SigningKeyStatus;
use rustycog::testing::http::jwt::{
    TEST_JWT_AUDIENCE, TEST_RS256_PRIVATE_PEM, TEST_RS256_PUBLIC_PEM,
};
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn expired_access_is_rejected_by_me_but_cannot_block_public_persisted_refresh() {
    let mut platform = registry::registry_key(SigningKeyStatus::Active, None);
    platform.kid = iam_domain::entity::signing_key::opaque_kid();
    let (fixture, base, client) = common::setup_test_server_with_signing_keys(&[platform.clone()])
        .await
        .expect("actual published RS256 platform registry");
    fixture_cleanup::run(&fixture, async {
    let email = "expired-access-refresh@example.com";
    let password = "ExpiredPublic1a!";
    let user = fixtures::DbFixtures::create_user_with_email_password(
        &fixture.db(),
        email,
        password,
        Some("expiredpublic"),
    )
    .await
    .expect("password account");
    let login = client
        .post(format!("{base}/api/auth/login"))
        .json(&json!({"email": email, "password": password}))
        .send()
        .await
        .expect("actual password login");
    assert_eq!(login.status(), 200);
    let login: Value = login.json().await.expect("login JSON");
    let refresh = login["refresh_token"]
        .as_str()
        .expect("persisted login refresh");
    let jwks = client
        .get(format!("{base}/.well-known/jwks.json"))
        .send()
        .await
        .expect("actual publisher");
    assert_eq!(jwks.status(), 200);
    let jwks: Value = jwks.json().await.expect("JWKS JSON");
    assert!(jwks["keys"]
        .as_array()
        .expect("keys")
        .iter()
        .any(|key| key["kid"] == platform.kid
            && key["status"] == "active"
            && key["trust_scope"] == "platform"));
    let extractor = rustycog::http::UserIdExtractor::from_inline_jwks(
        &jwks.to_string(),
        Some(TEST_JWT_AUDIENCE),
    )
    .expect("real publisher trust");
    extractor
        .extract_principal(login["access_token"].as_str().expect("actual access token"))
        .await
        .expect("positive published-key control");
    let now = chrono::Utc::now().timestamp();
    let claims = json!({"iss": platform.issuer, "sub": user.id().to_string(), "aud": TEST_JWT_AUDIENCE,
        "iat": now - 7200, "exp": now - 3600, "jti": uuid::Uuid::new_v4().to_string()});
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(platform.kid);
    header.typ = Some("aiforall-access+jwt".into());
    let expired = jsonwebtoken::encode(
        &header,
        &claims,
        &jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RS256_PRIVATE_PEM.as_bytes())
            .expect("fixed key"),
    )
    .expect("sign expired access");
    // Prove correct signature, kid-independent PEM material, issuer and audience;
    // only expiration is disabled for this diagnostic control, never in IAM.
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.validate_exp = false;
    validation.set_audience(&[TEST_JWT_AUDIENCE]);
    validation.set_issuer(&[claims["iss"].as_str().expect("issuer")]);
    jsonwebtoken::decode::<Value>(
        &expired,
        &jsonwebtoken::DecodingKey::from_rsa_pem(TEST_RS256_PUBLIC_PEM.as_bytes())
            .expect("fixed public key"),
        &validation,
    )
    .expect("valid signature and non-temporal claims");
    assert!(extractor.extract_principal(&expired).await.is_err());
    assert_eq!(
        client
            .get(format!("{base}/api/me"))
            .bearer_auth(&expired)
            .send()
            .await
            .expect("protected me")
            .status(),
        401
    );
    let response = client
        .post(format!("{base}/api/token/refresh"))
        .bearer_auth(&expired)
        .json(&json!({"refresh_token": refresh}))
        .send()
        .await
        .expect("public refresh with expired access");
    assert_eq!(response.status(), 200);
    let renewed: Value = response.json().await.expect("replacement JSON");
    let replacement = renewed["refresh_token"]
        .as_str()
        .expect("replacement refresh");
    assert!(
        replacement != refresh,
        "rotation must replace the actual session"
    );
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    let row = fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM refresh_tokens WHERE user_id=$1 AND token=$2 AND is_valid AND expires_at>NOW()",
        [user.id().into(), iam_domain::entity::token::RefreshToken::hash_token(replacement).into()]))
        .await.expect("persisted replacement").expect("count row");
    assert_eq!(row.try_get::<i64>("", "count").expect("count"), 1);
    }).await;
}
