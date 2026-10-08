//! Real Hive HTTP/PostgreSQL/OpenFGA gate; only the outbound IAM RPC is mocked.
//! Final-only testcontainers IT, never a Kind/protected-context harness.
mod common;

use common::{fixtures::db::DbFixtures, HiveTestFixture, Permission, ResourceRef, Subject};
use futures::FutureExt;
use rustycog::testing::{
    http::jwt::{
        create_jwt_token_with_secret, TEST_HS256_SECRET, TEST_JWT_AUDIENCE, TEST_JWT_ISSUER,
    },
    wiremock::MockServerFixture,
};
use serial_test::serial;
use std::{future::Future, panic::AssertUnwindSafe};
use uuid::Uuid;
use wiremock::{Mock, Request, ResponseTemplate};

const RPC_CREDENTIAL: &str = "SentinelABC-fixed-Hive-IAM-test-credential";

#[tokio::test]
#[serial]
async fn admin_transit_missing_or_zero_version_maps_iam400_without_hive_mutation_and_provider502_stays500(
) {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    let iam = IamSignerMockService::new().await;
    let (fixture, base, client, openfga, auth) =
        Box::pin(common::setup_test_server_with_iam_service(iam.config()))
            .await
            .unwrap();
    with_cleanup(&fixture,async {
        let owner=Uuid::new_v4();let org=DbFixtures::create_org_with_owner(fixture.db().as_ref(),owner).await.unwrap();
        let token=create_jwt_token_with_secret(owner,auth.jwt.hs256_secret.as_deref().unwrap());
        let endpoint=format!("{base}/api/organizations/{}/signer/configure",org.id);
        let state_sql=||Statement::from_sql_and_values(DatabaseBackend::Postgres,"SELECT to_jsonb(o) AS state FROM organizations o WHERE id=$1",[org.id.into()]);
        let before:serde_json::Value=fixture.db().query_one(state_sql()).await.unwrap().unwrap().try_get("","state").unwrap();
        for version in [None,Some(0u32)] {
            let mut body=serde_json::json!({"provider_type":"openbao_transit","provider_key_ref":format!("org-{}-key",org.id),"credential_ref":format!("org-{}-credential",org.id),"public_key":rustycog::testing::http::jwt::test_rs256_public_pem(),"org_slug":"spoofed"});
            if let Some(version)=version {body["provider_key_version"]=version.into();}
            openfga.deny(Subject::new(owner),Permission::Admin,ResourceRef::new("organization",org.id)).await.unwrap();
            let baseline=iam.received_requests().await.len();
            assert_eq!(client.post(&endpoint).bearer_auth(&token).json(&body).send().await.unwrap().status(),403);
            assert_eq!(iam.received_requests().await.len(),baseline,"permission denial must precede the IAM RPC even for malformed input");
            openfga.allow(Subject::new(owner),Permission::Admin,ResourceRef::new("organization",org.id)).await.unwrap();
            for (upstream,expected) in [(400,400),(502,500)] {
                iam.fixture.server().reset().await;
                Mock::given(wiremock::matchers::method("POST")).and(wiremock::matchers::path(format!("/iam/internal/organizations/{}/signer/configure",org.id)))
                    .respond_with(ResponseTemplate::new(upstream).set_body_string("private-provider-diagnostic")).expect(1).mount(&iam.fixture.server()).await;
                let response=client.post(&endpoint).bearer_auth(&token).json(&body).send().await.unwrap();assert_eq!(response.status().as_u16(),expected);
                if upstream==400 {assert!(!response.text().await.unwrap().contains("private-provider-diagnostic"));}
                let requests=iam.received_requests().await;assert_eq!(requests.len(),1);
                let sent:serde_json::Value=serde_json::from_slice(&requests[0].body).unwrap();assert_eq!(sent.get("provider_key_version").and_then(serde_json::Value::as_u64),version.map(u64::from));assert_eq!(sent["org_slug"],org.slug);
                let after:serde_json::Value=fixture.db().query_one(state_sql()).await.unwrap().unwrap().try_get("","state").unwrap();assert_eq!(after,before,"no Hive metadata persistence after IAM input/provider errors");
            }
        }
    }).await;
}

/// Typed HTTP collaborator local to this single newly authorized test file.
struct IamSignerMockService {
    fixture: MockServerFixture,
}
impl IamSignerMockService {
    async fn new() -> Self {
        Self {
            fixture: MockServerFixture::isolated().await,
        }
    }
    fn config(&self) -> hive_configuration::IamServiceConfig {
        hive_configuration::IamServiceConfig {
            base_url: self.fixture.base_url(),
            api_key: RPC_CREDENTIAL.into(),
            timeout_seconds: 5,
            workload: None,
        }
    }
    async fn received_requests(&self) -> Vec<Request> {
        self.fixture
            .server()
            .received_requests()
            .await
            .expect("outbound IAM request recording must be available")
    }
    async fn configure(&self, org: Uuid, profile: Uuid, issuer: &str) {
        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(format!("/iam/internal/organizations/{org}/signer/configure")))
            .and(wiremock::matchers::header("x-iam-internal-token",RPC_CREDENTIAL))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "signing_profile_id":profile,"kid":"fixed-outbound-test-kid","status":"active","issuer":issuer
            }))).mount(&self.fixture.server()).await;
    }
}

async fn with_cleanup<T>(fixture: &HiveTestFixture, body: impl Future<Output = T>) -> T {
    let result = AssertUnwindSafe(body).catch_unwind().await;
    let cleaned = common::cleanup_test_servers(fixture).await;
    if cleaned.is_err() {
        eprintln!("fixture-owned Hive listener join failed; parent lease reconciliation required");
    }
    match result {
        Ok(value) => {
            assert!(
                cleaned.is_ok(),
                "Hive fixture-owned listeners must join before lease release"
            );
            value
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[tokio::test]
#[serial]
async fn configure_requires_admin_on_the_exact_organization_before_any_iam_rpc() {
    let iam = IamSignerMockService::new().await;
    // Integration-owned additive helper: actual AppBuilder + same existing real
    // testcontainers/OpenFGA descriptor, isolated listener, only IAM config override.
    let (fixture, base, client, openfga, auth) =
        Box::pin(common::setup_test_server_with_iam_service(iam.config()))
            .await
            .expect("real isolated Hive HTTP harness with outbound IAM override");
    with_cleanup(&fixture, async {
        let owner = Uuid::new_v4();
        let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
            .await
            .expect("real organization/member/owner rows");
        let other = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
            .await
            .expect("separate organization scope");
        // Preserve the harness's explicit dual-verifier test policy. This request
        // uses its configured HS256 branch, never an arbitrary issuer or mesh metadata.
        assert_eq!(
            auth.jwt.jwks_url.as_deref(),
            Some("http://127.0.0.1:8081/iam/.well-known/jwks.json"),
            "this helper must report its actual explicit test verifier configuration"
        );
        assert_eq!(
            auth.jwt.allowed_algorithms,
            vec!["RS256".to_owned(), "HS256".to_owned()]
        );
        assert_eq!(auth.jwt.hs256_secret.as_deref(), Some(TEST_HS256_SECRET));
        assert_eq!(auth.jwt.issuer.as_deref(), Some(TEST_JWT_ISSUER));
        assert_eq!(auth.jwt.audience.as_deref(), Some(TEST_JWT_AUDIENCE));
        assert!(
            auth.mesh.trusted_gateway_san.is_empty(),
            "this is an actual Bearer guard test, not forged mesh principal metadata"
        );
        let token = create_jwt_token_with_secret(
            owner,
            auth.jwt
                .hs256_secret
                .as_deref()
                .expect("configured isolated test secret"),
        );
        let endpoint = format!("{base}/api/organizations/{}/signer/configure", org.id);
        let key_ref = format!("{}/fixture.pem", org.id);
        let body = serde_json::json!({"provider_type":"pem_file","provider_key_ref":key_ref,
            "provider_key_version":7,
            "public_key":rustycog::testing::http::jwt::test_rs256_public_pem(),
            "org_slug":other.slug,"organization_id":other.id});
        openfga
            .deny(
                Subject::new(owner),
                Permission::Admin,
                ResourceRef::new("organization", org.id),
            )
            .await
            .expect("explicit Admin DENY");
        let baseline = iam.received_requests().await.len();
        let denied = client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .expect("real denied configure request");
        assert_eq!(
            denied.status(),
            403,
            "a genuine authenticated member without Admin must be denied"
        );
        assert_eq!(
            iam.received_requests().await.len(),
            baseline,
            "Admin DENY must precede every outbound IAM call"
        );
        let profile = Uuid::new_v4();
        let issuer = format!("https://issuer.example/organizations/{}", org.slug);
        iam.configure(org.id, profile, &issuer).await;
        openfga
            .allow(
                Subject::new(owner),
                Permission::Admin,
                ResourceRef::new("organization", org.id),
            )
            .await
            .expect("ALLOW exact Admin organization tuple");
        let allowed = client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .expect("actual positive configure gate");
        assert_eq!(allowed.status(), 200);
        let allowed: serde_json::Value = allowed.json().await.expect("configure response");
        assert_eq!(allowed["signing_profile_id"], profile.to_string());
        assert_eq!(allowed["issuer"], issuer);
        let requests = iam.received_requests().await;
        assert_eq!(requests.len(), baseline + 1);
        let sent = requests.last().expect("positive outbound RPC");
        assert_eq!(sent.method.as_str(), "POST");
        assert_eq!(
            sent.url.path(),
            format!("/iam/internal/organizations/{}/signer/configure", org.id)
        );
        let sent_body: serde_json::Value =
            serde_json::from_slice(&sent.body).expect("typed outbound body");
        assert_eq!(
            sent_body["org_slug"], org.slug,
            "database slug must replace caller-controlled other-organization slug"
        );
        assert_eq!(sent_body["provider_key_ref"], key_ref);
        assert_eq!(
            sent_body["provider_key_version"], 7,
            "HTTP/command/outbound retain the explicit version"
        );
        assert_eq!(sent_body["public_key"], body["public_key"]);
        assert!(
            sent_body.get("organization_id").is_none(),
            "target comes from the authorized path, never arbitrary JSON"
        );
        let other_endpoint = format!("{base}/api/organizations/{}/signer/configure", other.id);
        openfga
            .deny(
                Subject::new(owner),
                Permission::Admin,
                ResourceRef::new("organization", other.id),
            )
            .await
            .expect("other organization remains DENY");
        assert_eq!(
            client
                .post(other_endpoint)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .expect("cross-org authorization")
                .status(),
            403
        );
        assert_eq!(
            iam.received_requests().await.len(),
            baseline + 1,
            "Admin on A cannot configure B"
        );
        openfga
            .deny(
                Subject::new(owner),
                Permission::Admin,
                ResourceRef::new("organization", org.id),
            )
            .await
            .expect("revoke exact grant");
        assert_eq!(
            client
                .post(&endpoint)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .expect("revoked Admin denial")
                .status(),
            403
        );
        assert_eq!(
            iam.received_requests().await.len(),
            baseline + 1,
            "revocation must deny before RPC again"
        );
    })
    .await;
}
