//! S9: actual facade/primary registry and outbound Transit, not a permissive registry.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use async_trait::async_trait;
use iam_application::usecase::organization_signer::{
    ConfigureOrganizationSignerInput, OrganizationSignerFacade, OrganizationSignerFacadeImpl,
};
use iam_domain::{
    error::DomainError,
    port::{WorkloadCredential, WorkloadIdentity},
};
use iam_infra::{
    repository::SeaOrmSigningKeyRegistry,
    signing::{
        DefaultOrganizationSignerProbe, DefaultOrganizationSignerRotator, RotateContext,
        StaticCredential, TransitClientConfig,
    },
};
use rustycog::testing::{
    http::jwt::{TEST_RS256_PRIVATE_PEM, TEST_RS256_PUBLIC_PEM},
    wiremock::MockServerFixture,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

struct ObservedCredentials {
    inner: StaticCredential,
    calls: AtomicUsize,
}
#[async_trait]
impl WorkloadIdentity for ObservedCredentials {
    async fn resolve(&self, name: &str) -> Result<WorkloadCredential, DomainError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.resolve(name).await
    }
}

#[tokio::test]
#[serial]
async fn missing_and_other_org_credentials_never_resolve_or_touch_vendor_but_valid_binding_configures(
) {
    let (fixture, _, _) = common::setup_test_server()
        .await
        .expect("primary registry harness");
    fixture_cleanup::run(&fixture, async {
        let vendor_fixture = MockServerFixture::isolated().await;
        let vendor = vendor_fixture.server();
        let org = Uuid::new_v4();
        let other = Uuid::new_v4();
        let key_name = format!("org-{org}-fixture-key");
        let credential = format!("org-{org}-fixture-credential");
        let workload = Arc::new(ObservedCredentials {
            inner: StaticCredential::from_pair(&credential, "SentinelABC-fixed-fixture-credential").expect("fixed test credential"),
            calls: AtomicUsize::new(0),
        });
        let registry = Arc::new(SeaOrmSigningKeyRegistry::new(
            fixture.db(),
            Arc::new(iam_domain::entity::signing_key::SigningKeyLifecyclePolicy::new()
                .expect("valid signing lifecycle policy")),
            iam_configuration::load_config_part::<iam_configuration::JwtConfig>("jwt")
                .expect("fixture JWT config").expiration_seconds,
        ).expect("valid writer registry"));
        let pem_root = tempfile::tempdir().expect("isolated PEM root");
        let transit = TransitClientConfig { base_url: vendor.uri(), workload: workload.clone(), token_ref: credential.clone() };
        let facade = OrganizationSignerFacadeImpl::new(registry.clone(), "https://issuer.example",
            Arc::new(DefaultOrganizationSignerProbe::new(pem_root.path().into()).with_transit(transit.clone())),
            Arc::new(DefaultOrganizationSignerRotator::new(RotateContext { registry, pem_root: pem_root.path().into(), transit: Some(transit) })));
        let mut input = ConfigureOrganizationSignerInput { provider_type:"openbao_transit".into(), provider_key_ref:key_name.clone(),
            credential_ref:None, public_key:TEST_RS256_PUBLIC_PEM.into(), org_slug:format!("fixture-{org}") };
        for reference in [None,Some(format!("org-{other}-fixture-credential")),Some(String::new())] {
            input.credential_ref = reference;
            assert!(matches!(facade.configure(org,&input).await,Err(DomainError::AuthorizationError(_))));
            assert_eq!(workload.calls.load(Ordering::SeqCst),0,"deny must precede credential resolution");
            assert_eq!(vendor.received_requests().await.expect("outbound audit").len(),0,"deny must precede vendor HTTP");
        }
        // Positive control is a genuine RSA response for the actual probe digest.
        use rsa::pkcs8::DecodePrivateKey;
        use sha2::Digest;
        use base64::Engine;
        let private = rsa::RsaPrivateKey::from_pkcs8_pem(TEST_RS256_PRIVATE_PEM).expect("fixed matching RSA pair");
        let signature = private.sign(rsa::pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>(), &sha2::Sha256::digest(b"aiforall-org-signer-challenge"))
            .expect("fixed fixture response signature");
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(format!("/v1/transit/sign/{key_name}")))
            .and(wiremock::matchers::header("x-vault-token","SentinelABC-fixed-fixture-credential"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{"signature":format!("vault:v1:{}",base64::engine::general_purpose::STANDARD.encode(signature))}})))
            .mount(&vendor).await;
        input.credential_ref = Some(credential);
        let configured = facade.configure(org,&input).await.expect("valid owned credential and cryptographic challenge");
        assert_eq!(configured.status,"active");
        assert_eq!(workload.calls.load(Ordering::SeqCst),1);
        assert_eq!(vendor.received_requests().await.expect("positive vendor audit").len(),1);
        let row = fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS count FROM signing_keys WHERE organization_id=$1 AND status='active'",[org.into()]))
            .await.expect("actual configured epoch").expect("count row");
        assert_eq!(row.try_get::<i64>("","count").expect("active count"),1);
    }).await;
}
