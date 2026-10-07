//! S9: actual facade/primary registry and outbound Transit, not a permissive registry.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/signing_prefill.rs"]
mod signing_state;
mod utils;

#[tokio::test]
#[serial]
async fn internal_http_transit_missing_or_zero_version_is400_without_any_signing_state_mutation() {
    let (fixture, base, client) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture,async {
        let org=Uuid::new_v4();let endpoint=format!("{base}/internal/organizations/{org}/signer/configure");
        let before=signing_state::database_state(fixture.db().as_ref()).await.unwrap();
        for version in [None,Some(0u32)] {
            let mut body=serde_json::json!({"provider_type":"openbao_transit","provider_key_ref":format!("org-{org}-key"),"credential_ref":format!("org-{org}-credential"),"public_key":TEST_RS256_PUBLIC_PEM,"org_slug":"fixture"});
            if let Some(version)=version {body["provider_key_version"]=version.into();}
            let response=client.post(&endpoint).header("x-iam-internal-token","iam-internal-test-token").json(&body).send().await.unwrap();
            assert_eq!(response.status(),400,"valid internal capability reaches input validation, not a provider failure");
            assert_eq!(signing_state::database_state(fixture.db().as_ref()).await.unwrap(),before);
        }
    }).await;
}

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
            Arc::new(DefaultOrganizationSignerProbe::new(pem_root.path().into()).with_local_pem_allowed(true).with_transit(transit.clone())),
            Arc::new(DefaultOrganizationSignerRotator::new(RotateContext { registry, pem_root: pem_root.path().into(), transit: Some(transit), allow_local_pem: true })));
        let mut input = ConfigureOrganizationSignerInput { provider_type:"openbao_transit".into(), provider_key_ref:key_name.clone(),
            provider_key_version: Some(1),
            credential_ref:None, public_key:TEST_RS256_PUBLIC_PEM.into(), org_slug:format!("fixture-{org}") };
        for reference in [None,Some(format!("org-{other}-fixture-credential")),Some(String::new())] {
            input.credential_ref = reference;
            assert!(matches!(facade.configure(org,&input).await,Err(DomainError::AuthorizationError(_))));
            assert_eq!(workload.calls.load(Ordering::SeqCst),0,"deny must precede credential resolution");
            assert_eq!(vendor.received_requests().await.expect("outbound audit").len(),0,"deny must precede vendor HTTP");
        }
        // Positive control is a genuine RSA response for the actual probe digest.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(format!("/v1/transit/keys/{key_name}")))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "type":"rsa-2048", "exportable":false, "supports_signing":true, "latest_version":2,
                    "keys": { "1": { "public_key": TEST_RS256_PUBLIC_PEM }, "2": { "public_key": TEST_RS256_PUBLIC_PEM } } }
            })))
            .mount(&vendor).await;
        use rsa::pkcs8::DecodePrivateKey;
        use sha2::Digest;
        use base64::Engine;
        let private = rsa::RsaPrivateKey::from_pkcs8_pem(TEST_RS256_PRIVATE_PEM).expect("fixed matching RSA pair");
        let signature = private.sign(rsa::pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>(), &sha2::Sha256::digest(b"aiforall-org-signer-challenge"))
            .expect("fixed fixture response signature");
        let signature = base64::engine::general_purpose::STANDARD.encode(signature);
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(format!("/v1/transit/sign/{key_name}")))
            .and(wiremock::matchers::header("x-vault-token","SentinelABC-fixed-fixture-credential"))
            .respond_with(move |request: &wiremock::Request| {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let version = body["key_version"].as_u64().unwrap();
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{"signature":format!("vault:v{version}:{signature}")}}))
            })
            .mount(&vendor).await;
        input.credential_ref = Some(credential);
        for version in [None, Some(0)] {
            input.provider_key_version = version;
            assert!(matches!(facade.configure(org, &input).await, Err(DomainError::InvalidSigningKeyMaterial)));
            assert_eq!(workload.calls.load(Ordering::SeqCst), 0);
            assert!(vendor.received_requests().await.expect("no vendor on invalid version").is_empty());
        }
        input.provider_key_version = Some(1);
        let configured = facade.configure(org,&input).await.expect("valid owned credential and cryptographic challenge");
        assert_eq!(configured.status,"active");
        assert_eq!(workload.calls.load(Ordering::SeqCst),2);
        let requests = vendor.received_requests().await.expect("positive vendor audit");
        assert_eq!(requests.len(),2, "exactly one public GET and one version-pinned Sign");
        assert_eq!(requests[0].method.as_str(), "GET");
        assert_eq!(requests[1].method.as_str(), "POST");
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&requests[1].body).expect("sign body")["key_version"], 1);
        let row = fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS count FROM signing_keys WHERE organization_id=$1 AND status='active'",[org.into()]))
            .await.expect("actual configured epoch").expect("count row");
        assert_eq!(row.try_get::<i64>("","count").expect("active count"),1);
        // Whole binding identical: no new probe, publication, kid or churn.
        let repeated = facade.configure(org, &input).await.unwrap();
        assert_eq!(repeated.kid, configured.kid);
        assert_eq!(workload.calls.load(Ordering::SeqCst), 2);
        assert_eq!(vendor.received_requests().await.unwrap().len(), 2);
        // Version alone is an effective change even with identical public n/e.
        // Distinct materials for real rotation are tested separately.
        input.provider_key_version = Some(2);
        let second = facade.configure(org, &input).await.unwrap();
        assert_ne!(second.kid, configured.kid);
        assert_eq!(second.status, "active");
        assert_eq!(workload.calls.load(Ordering::SeqCst), 4);
        let rows = fixture.db().query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT kid,status,provider_key_version FROM signing_keys WHERE organization_id=$1 ORDER BY provider_key_version", [org.into()])).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].try_get::<String>("", "status").unwrap(), "retiring");
        assert_eq!(rows[0].try_get::<i64>("", "provider_key_version").unwrap(), 1);
        assert_eq!(rows[1].try_get::<String>("", "status").unwrap(), "active");
        assert_eq!(rows[1].try_get::<i64>("", "provider_key_version").unwrap(), 2);
        let requests = vendor.received_requests().await.unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&requests[3].body).unwrap()["key_version"], 2);
        assert!(requests.iter().all(|request| !request.url.path().ends_with("/rotate")), "configure must not invent a provider rotation");
        // Metadata refusal before PoP/prepare leaves the old Active and admission
        // history intact. A different pin prevents the no-op branch masking this.
        input.provider_key_version = Some(3);
        for case in 0..8 {
            let mut data = serde_json::json!({"type":"rsa-2048", "exportable":false, "supports_signing":true,
                "latest_version":3, "keys":{"3":{"public_key":TEST_RS256_PUBLIC_PEM}}});
            match case {
                0 => { data.as_object_mut().unwrap().remove("supports_signing"); }
                1 => data["supports_signing"] = false.into(),
                2 => { data.as_object_mut().unwrap().remove("exportable"); }
                3 => data["exportable"] = true.into(),
                4 => data["type"] = "ecdsa-p256".into(),
                5 => data["type"] = "rsa-4096".into(),
                6 => { data.as_object_mut().unwrap().remove("latest_version"); }
                _ => data["latest_version"] = 0.into(),
            }
            let guard = wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::path(format!("/v1/transit/keys/{key_name}")))
                .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":data}))).with_priority(1)
                .mount_as_scoped(&vendor).await;
            let before_requests = vendor.received_requests().await.unwrap().len();
            assert!(facade.configure(org, &input).await.is_err());
            let after_requests = vendor.received_requests().await.unwrap();
            assert_eq!(after_requests.len(), before_requests + 1);
            assert_eq!(after_requests.last().unwrap().method.as_str(), "GET");
            let state = fixture.db().query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT kid,status,provider_key_version FROM signing_keys WHERE organization_id=$1 ORDER BY provider_key_version", [org.into()])).await.unwrap();
            assert_eq!(state.len(), 2, "invalid metadata must not admit Pending");
            assert_eq!(state[0].try_get::<String>("", "status").unwrap(), "retiring");
            assert_eq!(state[1].try_get::<String>("", "kid").unwrap(), second.kid);
            assert_eq!(state[1].try_get::<String>("", "status").unwrap(), "active");
            drop(guard);
        }
    }).await;
}
