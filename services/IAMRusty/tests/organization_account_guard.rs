//! S1: an actually published/trusted org signer must not authorize an IAM account.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../../../workers/ext-authz/tests/support/registry.rs"]
mod registry;
mod utils;

use fixtures::{DbFixtures, IdpConnectFixtures};
use iam_domain::entity::signing_key::SigningKeyStatus;
use rustycog::{
    http::UserIdExtractor,
    testing::http::jwt::{test_rs256_private_pem, TEST_JWT_AUDIENCE},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

fn sign(key: &iam_domain::entity::signing_key::SigningKey, victim: Uuid) -> String {
    sign_with_pem(key, victim, test_rs256_private_pem())
}

fn sign_with_pem(
    key: &iam_domain::entity::signing_key::SigningKey,
    victim: Uuid,
    private: &str,
) -> String {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(key.kid.clone());
    header.typ = Some("aiforall-access+jwt".into());
    let now = chrono::Utc::now().timestamp();
    let mut claims = json!({"sub": victim.to_string(), "iss": key.issuer, "aud": TEST_JWT_AUDIENCE,
        "iat": now, "exp": now + 3600, "jti": Uuid::new_v4().to_string()});
    if let Some(org) = key.organization_id {
        claims["org"] = org.to_string().into();
    }
    jsonwebtoken::encode(
        &header,
        &claims,
        &jsonwebtoken::EncodingKey::from_rsa_pem(private.as_bytes()).expect("fixed nonsecret key"),
    )
    .expect("signed principal")
}

#[tokio::test]
#[serial]
async fn published_organization_signer_with_victim_sub_cannot_me_link_or_relink() {
    let mut platform = registry::registry_key(SigningKeyStatus::Active, None);
    platform.kid = iam_domain::entity::signing_key::opaque_kid();
    let mut organization = registry::registry_key(SigningKeyStatus::Active, Some(Uuid::new_v4()));
    organization.kid = iam_domain::entity::signing_key::opaque_kid();
    // Integration must seed REAL writer registry rows, configure RS256/JWKS trust,
    // bind platform issuer from this row and use the actual GET publisher route.
    let (fixture, base, client) = Box::pin(common::setup_test_server_with_signing_keys(&[
        platform.clone(),
        organization.clone(),
    ]))
    .await
    .expect("RS256 IAM registry harness");
    fixture_cleanup::run(&fixture, async {
        let victim = DbFixtures::user()
            .arthur()
            .commit(fixture.db())
            .await
            .expect("victim account");
        let org_token = sign(&organization, victim.id());
        let platform_token = sign(&platform, victim.id());
        let jwks = client
            .get(format!("{base}/.well-known/jwks.json"))
            .send()
            .await
            .expect("real publisher GET");
        assert_eq!(jwks.status(), 200);
        let jwks: Value = jwks.json().await.expect("published JWKS");
        let org_key = jwks["keys"]
            .as_array()
            .expect("keys")
            .iter()
            .find(|key| key["kid"] == organization.kid)
            .expect("real org key published");
        assert_eq!(org_key["trust_scope"], "organization");
        assert_eq!(
            org_key["organization_id"],
            organization.organization_id.expect("org").to_string()
        );
        let extractor =
            UserIdExtractor::from_inline_jwks(jwks.to_string(), Some(TEST_JWT_AUDIENCE))
                .expect("published snapshot");
        let principal = extractor
            .extract_principal(&org_token)
            .await
            .expect("legitimate org principal must remain trusted");
        assert_eq!(principal.sub, victim.id());
        assert_eq!(principal.iss, organization.issuer);
        // A valid local platform principal still reaches the same account route.
        assert_eq!(
            client
                .get(format!("{base}/api/me"))
                .bearer_auth(&platform_token)
                .send()
                .await
                .expect("platform me")
                .status(),
            200
        );
        let idp = IdpConnectFixtures::service().await;
        idp.mock_authorize("github").await;
        let before = idp.received_requests().await.len();
        for route in ["me", "auth/github/link", "auth/github/relink-start"] {
            let response = client
                .get(format!("{base}/api/{route}"))
                .bearer_auth(&org_token)
                .send()
                .await
                .expect("org account attempt");
            assert_eq!(
                response.status(),
                403,
                "trusted org token must fail the platform account guard on {route}"
            );
            assert_eq!(
                idp.received_requests().await.len(),
                before,
                "account guard must precede IdP authorization"
            );
        }
        let row = fixture
            .db()
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS count FROM oauth_transactions WHERE target_user_id = $1",
                [victim.id().into()],
            ))
            .await
            .expect("inspect transaction mutation")
            .expect("count row");
        assert_eq!(row.try_get::<i64>("", "count").expect("count"), 0);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn post_boot_published_kid_resolves_through_live_url_refresh() {
    use iam_domain::port::repository::SigningKeyRegistry;
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    let (fixture, base, client) = Box::pin(common::setup_test_server())
        .await
        .expect("real IAM listener boot");
    fixture_cleanup::run(&fixture, async {
        let victim = DbFixtures::user().arthur().commit(fixture.db()).await.expect("victim account");
        let (writer, _) = common::fixture_signing_registry(&fixture).expect("same primary writer");
        // Generate and admit AFTER boot: neither kid nor RSA material can be in
        // the boot seed. No warmup bearer or arbitrary 60-second wait is used.
        let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("distinct post-boot RSA");
        let mut organization = registry::registry_key(SigningKeyStatus::Active, Some(Uuid::new_v4()));
        organization.kid = iam_domain::entity::signing_key::opaque_kid();
        organization.public_key = private.to_public_key().to_public_key_pem(LineEnding::LF).expect("new SPKI");
        let before = writer.jwks_publication_snapshot().await.expect("primary state after boot");
        let new_public=iam_domain::entity::token::Jwk::from_rsa_pem(&organization.public_key,&organization.kid,&organization.issuer).unwrap();
        assert!(before.publication.jwks().keys.iter().all(|key| key.kid != organization.kid && (key.n != new_public.n || key.e != new_public.e)));
        let admitted = writer.replace_active_organization_key(&organization, None).await.expect("real post-boot admission");
        let private_pem = private.to_pkcs8_pem(LineEnding::LF).expect("fixture private material");
        let token = sign_with_pem(&admitted, victim.id(), private_pem.as_str());
        let idp = IdpConnectFixtures::service().await;
        let idp_before = idp.received_requests().await.len();
        let response = client.get(format!("{base}/api/me")).bearer_auth(&token)
            .send().await.expect("first bearer with post-boot kid");
        assert_eq!(response.status(), 403,
            "a post-boot organization kid must resolve via the live URL and reach the account guard, not fail 401");
        assert_eq!(idp.received_requests().await.len(), idp_before, "guard precedes IdP");
    }).await;
}
