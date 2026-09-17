//! Apparatus P3 — T9 public invoke path on the prefixed router.

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use apparatus_contracts::{new_operation_id, BindingId, InvokeRequest};
use common::LazaretTestDescriptor;
use fixtures::BindingSnapshotFixtures;
use lazaret_application::EnrollCommand;
use lazaret_configuration::{load_config, ConnectorEntry};
use lazaret_domain::{
    BindingGrantSnapshot, CapabilityConsent, PrincipalMembership, WorkloadIdentity,
};
use lazaret_setup::Application;
use rcgen::{CertificateParams, KeyPair};
use rustycog::testing::{ServiceTestDescriptor, TestFixture};
use serial_test::serial;
use uuid::Uuid;

fn workload_csr() -> String {
    let key = KeyPair::generate().expect("key");
    let params = CertificateParams::new(vec!["workload.test".to_owned()]).expect("params");
    params
        .serialize_request(&key)
        .expect("csr")
        .pem()
        .expect("pem")
}

async fn enroll_session(app: &Application, project_id: Uuid, identity: WorkloadIdentity) -> String {
    let csr_pem = workload_csr();
    let issued = app
        .identity
        .enroll(EnrollCommand {
            csr_pem,
            identity,
            project_id,
        })
        .await
        .expect("enroll");
    let cert = lazaret_domain::VerifiedClientCertificate::from_pem(&issued.pem).expect("cert");
    app.identity.issue_session(&cert).await.expect("session")
}

fn snapshot(
    project_id: Uuid,
    binding: Uuid,
    grant_revision: i64,
    caps: &[&str],
    principal: Option<PrincipalMembership>,
) -> BindingGrantSnapshot {
    BindingGrantSnapshot {
        component_id: binding,
        project_id,
        project_status: "active".to_owned(),
        component_status: "active".to_owned(),
        source: "managed".to_owned(),
        digest: Some("release-1".to_owned()),
        desired_generation: 1,
        observed_generation: 1,
        grant_revision,
        declared: caps.iter().map(|name| (*name).to_owned()).collect(),
        consents: caps
            .iter()
            .map(|name| CapabilityConsent {
                capability: (*name).to_owned(),
                status: "consented".to_owned(),
                grant_revision,
            })
            .collect(),
        principal,
    }
}

fn invoke_body(binding: Uuid, operation: &str, params: serde_json::Value) -> InvokeRequest {
    InvokeRequest {
        binding_id: binding.to_string().parse::<BindingId>().expect("id"),
        operation_id: new_operation_id(),
        operation: operation.to_owned(),
        params,
    }
}

async fn boot(
    manifesto_url: String,
    connectors: Vec<ConnectorEntry>,
) -> (TestFixture, Application) {
    let descriptor = Arc::new(LazaretTestDescriptor);
    let fixture = TestFixture::new(descriptor).await.expect("fixture");
    let mut config = load_config().expect("config");
    config.manifesto_service.base_url = manifesto_url;
    config.connectors = connectors;
    let app = Application::new(config).await.expect("app");
    (fixture, app)
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t9_public_lazaret_invoke_on_prefixed_router_is_200() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        binding,
        snapshot(
            project_id,
            binding,
            1,
            &["storage.kv.read", "storage.kv.write"],
            None,
        ),
    )
    .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.prefixed_router()).expect("test server");

    let put = server
        .post(&format!("/lazaret/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1"}),
        ))
        .await;
    assert_eq!(put.status_code(), 200, "{}", put.text());

    let get = server
        .post(&format!("/lazaret/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(get.status_code(), 200);
    let body: serde_json::Value = get.json();
    assert_eq!(body["result"]["value"], "v1");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t9_unprefixed_invoke_on_prefixed_router_is_404() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        binding,
        snapshot(
            project_id,
            binding,
            1,
            &["storage.kv.read", "storage.kv.write"],
            None,
        ),
    )
    .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.prefixed_router()).expect("test server");

    let unprefixed = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1"}),
        ))
        .await;
    assert_eq!(unprefixed.status_code(), 404, "{}", unprefixed.text());

    let double_nest = server
        .post(&format!("/lazaret/lazaret/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1"}),
        ))
        .await;
    assert_eq!(double_nest.status_code(), 404, "{}", double_nest.text());

    let trailing = server
        .post(&format!("/lazaret/invoke/?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1"}),
        ))
        .await;
    assert_eq!(trailing.status_code(), 404, "{}", trailing.text());
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t9_run_wiring_unauthenticated_invoke_is_401() {
    let (_fixture, base_url, client) = common::setup_test_server().await.expect("serveur de test");
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let response = client
        .post(format!("{base_url}/invoke?project_id={project_id}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .send()
        .await
        .expect("POST /lazaret/invoke without auth");
    assert_eq!(
        response.status(),
        401,
        "invoke must be mounted on run() wiring, got {}",
        response.status()
    );
}
