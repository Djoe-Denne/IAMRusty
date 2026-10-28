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

struct RecognizedAuthority {
    issuer: String,
    organization: Option<Uuid>,
    kid: String,
    jwk: serde_json::Value,
    encoding: jsonwebtoken::EncodingKey,
}

impl RecognizedAuthority {
    fn new(issuer: &str, organization: Option<Uuid>) -> Self {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use rsa::{
            pkcs8::{EncodePrivateKey, LineEnding},
            traits::PublicKeyParts,
        };
        use rustycog::testing::http::jwt::CanonicalJwk;
        let private =
            rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("local test RSA key");
        let public = rsa::RsaPublicKey::from(&private);
        let canonical = match organization {
            Some(id) => CanonicalJwk::organization(issuer, id),
            None => CanonicalJwk::platform(issuer),
        };
        let kid = canonical.kid().to_owned();
        let mut jwk = serde_json::to_value(canonical).unwrap();
        jwk["n"] = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()).into();
        jwk["e"] = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()).into();
        let pem = private.to_pkcs8_pem(LineEnding::LF).unwrap();
        let encoding = jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();
        Self {
            issuer: issuer.into(),
            organization,
            kid,
            jwk,
            encoding,
        }
    }

    fn token(&self, subject: Uuid) -> String {
        let now = chrono::Utc::now().timestamp();
        let mut claims = serde_json::json!({
            "iss": self.issuer, "sub": subject, "aud": TEST_JWT_AUDIENCE,
            "iat": now, "exp": now + 300, "jti": Uuid::new_v4(),
        });
        if let Some(organization) = self.organization {
            claims["org"] = organization.to_string().into();
        }
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        header.typ = Some(rustycog::http::ACCESS_TOKEN_TYP.into());
        jsonwebtoken::encode(&header, &claims, &self.encoding).unwrap()
    }
}

async fn signer_writer_snapshot(
    fixture: &HiveTestFixture,
    organization: Uuid,
) -> serde_json::Value {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('organization', (SELECT to_jsonb(o) FROM organizations o WHERE o.id=$1), 'members', (SELECT COALESCE(jsonb_agg(to_jsonb(m) ORDER BY m.id), '[]'::jsonb) FROM organization_members m WHERE m.organization_id=$1), 'outbox_count', (SELECT COUNT(*) FROM rustycog_outbox_events)) AS state",
        [organization.into()])).await.unwrap().unwrap().try_get("", "state").unwrap()
}

#[tokio::test]
#[serial]
async fn recognized_same_sub_distinct_issuers_do_not_share_organization_admin() {
    use rustycog::{
        config::{AuthConfig, JwtAuthConfig},
        permission::{PrincipalPolicy, PrincipalRef},
    };
    let platform = RecognizedAuthority::new("https://platform.example/iam", None);
    let org1 = RecognizedAuthority::new("https://org1.example/iam", Some(Uuid::new_v4()));
    let org2 = RecognizedAuthority::new("https://org2.example/iam", Some(Uuid::new_v4()));
    let jwks = MockServerFixture::isolated().await;
    Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"keys": [platform.jwk, org1.jwk, org2.jwk]})),
        )
        .mount(&jwks.server())
        .await;
    let auth = AuthConfig {
        jwt: JwtAuthConfig {
            platform_issuer: Some(platform.issuer.clone()),
            allow_historical_platform_issuer: Some(false),
            issuer: Some(platform.issuer.clone()),
            audience: Some(TEST_JWT_AUDIENCE.into()),
            jwks_url: Some(format!("{}/jwks", jwks.base_url())),
            allowed_algorithms: vec!["RS256".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    let same_sub = Uuid::new_v4();
    let tokens = [
        platform.token(same_sub),
        org1.token(same_sub),
        org2.token(same_sub),
    ];
    let policy = PrincipalPolicy::new(&platform.issuer, false).unwrap();
    let authorities = [&platform, &org1, &org2];
    let subjects: Vec<_> = authorities
        .iter()
        .map(|authority| {
            Subject::from_principal(
                PrincipalRef::new(&authority.issuer, same_sub).unwrap(),
                &policy,
            )
            .unwrap()
        })
        .collect();
    // Recognition is positive and independent of permissions. A 401 must never
    // masquerade as a successful collision denial.
    let verifier = rustycog::http::UserIdExtractor::new(auth.clone()).unwrap();
    for (authority, token) in authorities.iter().zip(tokens.iter()) {
        let principal = verifier
            .extract_principal(token)
            .await
            .expect("each independent JWT must be recognized");
        assert_eq!(principal.sub, same_sub);
        assert_eq!(principal.iss, authority.issuer);
    }
    let iam = IamSignerMockService::new().await;
    let (fixture, base, client, openfga, _) = Box::pin(
        common::setup_test_server_with_auth_and_iam(iam.config(), Some(auth)),
    )
    .await
    .unwrap();
    with_cleanup(&fixture, async {
        let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), same_sub).await.unwrap();
        let other = DbFixtures::create_org_with_owner(fixture.db().as_ref(), same_sub).await.unwrap();
        let resource = ResourceRef::new("organization", organization.id);
        let endpoint = format!("{base}/api/organizations/{}/signer/configure", organization.id);
        let body = serde_json::json!({
            "provider_type": "pem_file", "provider_key_ref": format!("{}/fixture.pem", organization.id),
            "provider_key_version": 7, "public_key": rustycog::testing::http::jwt::test_rs256_public_pem(),
            "organization_id": other.id, "org_slug": other.slug,
        });
        iam.configure(organization.id, Uuid::new_v4(), &platform.issuer).await;
        openfga.allow(subjects[0].clone(), Permission::Admin, resource).await.unwrap();
        let positive = client.post(&endpoint).bearer_auth(&tokens[0]).json(&body).send().await.unwrap();
        assert_eq!(positive.status(), 200);
        assert_eq!(iam.received_requests().await.len(), 1);
        for index in [1usize, 2] {
            let before = signer_writer_snapshot(&fixture, organization.id).await;
            let rpc_before = iam.received_requests().await.len();
            let response = client.post(&endpoint).bearer_auth(&tokens[index]).json(&body).send().await.unwrap();
            assert_eq!(response.status(), 403, "recognized foreign principal must reach a denial, never an authentication rejection");
            assert_eq!(iam.received_requests().await.len(), rpc_before, "denial must precede IAM RPC");
            assert!(before == signer_writer_snapshot(&fixture, organization.id).await, "denial must not mutate Hive/outbox");
        }
        // Each organization authority can act with its own legitimate grant.
        // Revoking it never revokes the platform or the other organization.
        for index in [1usize, 2] {
            openfga.allow(subjects[index].clone(), Permission::Admin, resource).await.unwrap();
            let rpc_before = iam.received_requests().await.len();
            assert_eq!(client.post(&endpoint).bearer_auth(&tokens[index]).json(&body).send().await.unwrap().status(), 200);
            assert_eq!(iam.received_requests().await.len(), rpc_before + 1);
        }
        openfga.deny(subjects[1].clone(), Permission::Admin, resource).await.unwrap();
        let before = signer_writer_snapshot(&fixture, organization.id).await;
        let rpc_before = iam.received_requests().await.len();
        assert_eq!(client.post(&endpoint).bearer_auth(&tokens[1]).json(&body).send().await.unwrap().status(), 403);
        assert_eq!(iam.received_requests().await.len(), rpc_before);
        assert!(before == signer_writer_snapshot(&fixture, organization.id).await, "revoked principal must not mutate writer/outbox");
        for index in [0usize, 2] {
            assert_eq!(client.post(&endpoint).bearer_auth(&tokens[index]).json(&body).send().await.unwrap().status(), 200);
        }
        for request in iam.received_requests().await {
            assert_eq!(request.url.path(), format!("/iam/internal/organizations/{}/signer/configure", organization.id));
            let sent: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(sent["org_slug"], organization.slug, "body organization must not replace exact authorized path");
        }
    }).await;
}

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
