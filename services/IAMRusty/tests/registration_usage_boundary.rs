//! §13: actual registration issuance/completion, never an IAM account Bearer.
#[path = "support/browser_flow.rs"]
mod browser_flow;
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../../../workers/ext-authz/tests/support/registry.rs"]
mod registry;
mod utils;

use iam_domain::entity::signing_key::SigningKeyStatus;
use rustycog::testing::http::jwt::test_rs256_public_pem;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn password_and_oauth_registration_preserve_real_24h_claims_and_cannot_be_account_bearers() {
    let mut key = registry::registry_key(SigningKeyStatus::Active, None);
    key.kid = iam_domain::entity::signing_key::opaque_kid();
    let (fixture, base, client) =
        Box::pin(common::setup_test_server_with_signing_keys(&[key.clone()]))
            .await
            .expect("actual writer-bound RS256 registration codec");
    fixture_cleanup::run(&fixture, async {
        use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
        let signup = client.post(format!("{base}/api/auth/signup"))
            .json(&json!({"email":"registration-boundary@example.com", "password":"Boundary1a!Password"}))
            .send().await.expect("actual signup");
        assert_eq!(signup.status(), 202);
        let password: Value = signup.json().await.expect("signup JSON");
        let idp = fixtures::IdpConnectFixtures::service().await;
        idp.mock_gitlab_happy_alice().await;
        let (state, cookie) = browser_flow::login(&client, &base, "gitlab", &idp).await;
        let oauth = client.get(format!("{base}/api/auth/gitlab/callback"))
            .header("cookie", &cookie).query(&[("code","test_auth_code"),("state",state.as_str())])
            .send().await.unwrap_or_else(|_| panic!("callback transport failed (state URL redacted)"));
        assert_eq!(oauth.status(), 202);
        let oauth: Value = oauth.json().await.expect("OAuth registration JSON");
        let published = client.get(format!("{base}/.well-known/jwks.json")).send().await.expect("actual publisher");
        assert_eq!(published.status(), 200);
        let published: Value = published.json().await.expect("JWKS");
        assert!(published["keys"].as_array().expect("keys").iter().any(|jwk| jwk["kid"]==key.kid && jwk["iss"]==key.issuer));
        let extractor = rustycog::http::UserIdExtractor::from_inline_jwks(published.to_string(), Some("aiforall"))
            .expect("publisher-bound account verifier");
        for (result, flow, username) in [(&password,"email_password","regpassword"),(&oauth,"oauth","regoauth")] {
            let token = result["registration_token"].as_str().expect("real registration issuance");
            let header = jsonwebtoken::decode_header(token).expect("registration header");
            assert_eq!(header.alg, jsonwebtoken::Algorithm::RS256);
            assert_eq!(header.kid.as_deref(), Some(key.kid.as_str()));
            assert_ne!(header.typ.as_deref(), Some("aiforall-access+jwt"));
            let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
            validation.set_audience(&["registration"]);
            validation.set_issuer(&[key.issuer.as_str()]);
            validation.sub = Some("registration".into());
            let claims = jsonwebtoken::decode::<Value>(token,
                &jsonwebtoken::DecodingKey::from_rsa_pem(test_rs256_public_pem().as_bytes()).expect("published public material"), &validation)
                .expect("genuinely valid registration signature/kid bindings and registration claims").claims;
            assert_eq!(claims["flow"], flow);
            assert_eq!(claims["exp"].as_i64().expect("exp")-claims["iat"].as_i64().expect("iat"), 86400);
            assert!(claims["jti"].as_str().is_some_and(|v| !v.is_empty()));
            assert!(claims["email"].as_str().is_some_and(|v| !v.is_empty()));
            if flow=="oauth" { assert!(claims["provider_info"].is_object()); }
            assert!(extractor.extract_principal(token).await.is_err(), "valid registration must not become a principal");
            for route in ["me","auth/github/link","auth/github/relink-start"] {
                assert_eq!(client.get(format!("{base}/api/{route}")).bearer_auth(token)
                    .send().await.expect("real account guard").status(),401);
            }
            // Verification is fixture arrangement, never an inserted auth session.
            // Completion may emit access only for a genuinely verified-email row.
            let verified = fixture.db().execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE user_emails SET is_verified=true WHERE email=$1", [claims["email"].as_str().expect("email").into()]))
                .await.expect("verified-email arrangement");
            assert_eq!(verified.rows_affected(),1);
            let complete = client.post(format!("{base}/api/auth/complete-registration"))
                .json(&json!({"registration_token":token,"username":username})).send().await.expect("actual completion");
            assert_eq!(complete.status(),200,"dedicated registration use must remain valid");
            let complete: Value = complete.json().await.expect("completion JSON");
            let access = complete["access_token"].as_str().expect("account access after completion");
            extractor.extract_principal(access).await.expect("same published key admits actual account access");
            assert_eq!(client.get(format!("{base}/api/me")).bearer_auth(access).send().await.expect("account positive control").status(),200);
            assert!(client.post(format!("{base}/api/auth/complete-registration"))
                .json(&json!({"registration_token":access,"username":"mustnotpromoteaccess"})).send().await.expect("access is not registration")
                .status().is_client_error());
        }
    }).await;
}
