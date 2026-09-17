//! Apparatus P3 — T7 HTTP `/invoke` bound to the current binding.

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use apparatus_contracts::{new_operation_id, BindingId, InvokeRequest, MAX_PAYLOAD_BYTES};
use common::LazaretTestDescriptor;
use fixtures::{BindingSnapshotFixtures, VaultFixtures};
use lazaret_application::EnrollCommand;
use lazaret_configuration::{load_config, ConnectorEntry};
use lazaret_domain::{
    BindingGrantSnapshot, CapabilityConsent, ConnectorRegistry, PrincipalMembership,
    WorkloadIdentity,
};
use lazaret_infra::NamedConnectorProxy;
use lazaret_setup::Application;
use rcgen::{CertificateParams, KeyPair};
use rustycog::testing::{ServiceTestDescriptor, TestFixture};
use serial_test::serial;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

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
async fn t7_invoke_kv_ops_require_live_grant_and_session() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let user = Uuid::new_v4();
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
    let server = axum_test::TestServer::new(app.router()).expect("test server");

    let put = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1"}),
        ))
        .await;
    assert_eq!(put.status_code(), 200, "{}", put.text());

    let get = server
        .post(&format!("/invoke?project_id={project_id}"))
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

    let iam = rustycog::testing::http::jwt::create_jwt_token(user);
    let unauth = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {iam}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(unauth.status_code(), 401);
    let requests = mock.received_requests().await;
    assert!(
        requests
            .iter()
            .all(|req| { !req.url.query().unwrap_or("").contains("principal") }),
        "invoke must not send principal query"
    );
    assert!(
        requests
            .iter()
            .any(|req| req.headers.get("authorization").is_some()),
        "signed consult must send Authorization"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_stale_revision_and_foreign_binding_and_ungranted_op_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let other = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 0)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        binding,
        snapshot(project_id, binding, 0, &["storage.kv.read"], None),
    )
    .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    mock.reset().await;
    mock.mock_get_snapshot_background(
        project_id,
        binding,
        snapshot(project_id, binding, 2, &["storage.kv.read"], None),
    )
    .await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");

    let stale = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(stale.status_code(), 403);

    let switch = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            other,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(switch.status_code(), 403);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_cross_project_and_ungranted_capability_denied() {
    let project_a = Uuid::new_v4();
    let project_b = Uuid::new_v4();
    let binding_a = Uuid::new_v4();
    let binding_b = Uuid::new_v4();
    let identity =
        WorkloadIdentity::try_new(Uuid::new_v4(), binding_a, "release-1".to_owned(), 1, 1)
            .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_a,
        binding_a,
        snapshot(project_a, binding_a, 1, &["storage.kv.read"], None),
    )
    .await;
    mock.mock_get_snapshot_background(
        project_b,
        binding_b,
        snapshot(
            project_b,
            binding_b,
            1,
            &["storage.kv.read", "storage.kv.write"],
            None,
        ),
    )
    .await;
    mock.mock_get_snapshot_background(
        project_b,
        binding_a,
        snapshot(project_a, binding_a, 1, &["storage.kv.read"], None),
    )
    .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_a, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");

    let other_project = server
        .post(&format!("/invoke?project_id={project_b}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding_a,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(other_project.status_code(), 403);

    let other_binding = server
        .post(&format!("/invoke?project_id={project_b}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding_b,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(other_binding.status_code(), 403);

    let ungranted = server
        .post(&format!("/invoke?project_id={project_a}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding_a,
            "kv.put",
            serde_json::json!({"key": "k", "value": "x"}),
        ))
        .await;
    assert_eq!(ungranted.status_code(), 403);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_named_connector_rejects_raw_url_and_allows_admitted_name() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let snapshot_mock = BindingSnapshotFixtures::service().await;
    snapshot_mock
        .mock_get_snapshot_background(
            project_id,
            binding,
            snapshot(project_id, binding, 1, &["connector.fetch"], None),
        )
        .await;

    let connector_mock = rustycog::testing::wiremock::MockServerFixture::isolated().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
        .mount(&*connector_mock.server())
        .await;

    let (_fixture, app) = boot(
        snapshot_mock.base_url(),
        vec![ConnectorEntry {
            name: "docs".to_owned(),
            url: connector_mock.server().uri(),
        }],
    )
    .await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");

    let raw = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "connector.fetch",
            serde_json::json!({"url": "http://127.0.0.1:1/", "path": "/"}),
        ))
        .await;
    assert_eq!(raw.status_code(), 400);

    let internal = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "connector.fetch",
            serde_json::json!({"fetchInternal": true, "connector": "docs"}),
        ))
        .await;
    assert_eq!(internal.status_code(), 400);

    let ok = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "connector.fetch",
            serde_json::json!({"connector": "docs", "path": "/hello"}),
        ))
        .await;
    assert_eq!(ok.status_code(), 200, "{}", ok.text());
    let body: serde_json::Value = ok.json();
    assert_eq!(body["result"]["body"], "pong");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_public_project_without_consent_does_not_open_call() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 0)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    let mut empty = snapshot(project_id, binding, 0, &[], None);
    empty.project_status = "active".to_owned();
    mock.mock_get_snapshot_background(project_id, binding, empty)
        .await;
    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");
    let resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(resp.status_code(), 403);
}

#[test]
fn t7_registry_rejects_url_shaped_names() {
    let registry =
        ConnectorRegistry::from_entries(&[("docs".to_owned(), "https://example.com".to_owned())]);
    assert!(registry.lookup("http://169.254.169.254").is_err());
    assert!(registry.lookup("fetchInternal").is_err());
    assert!(registry.join_path("docs", "https://evil").is_err());
    assert!(registry.join_path("docs", "/../secret").is_err());
    assert!(NamedConnectorProxy::new(registry).is_ok());
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_manifesto_404_is_403_without_principal_query() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        binding,
        snapshot(project_id, binding, 1, &["storage.kv.read"], None),
    )
    .await;
    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    mock.reset().await;
    mock.mock_get_snapshot_not_found(project_id, binding).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");
    let resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(resp.status_code(), 403);
    let requests = mock.received_requests().await;
    assert!(
        requests
            .iter()
            .all(|req| !req.url.query().unwrap_or("").contains("principal")),
        "no principal query"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_secret_reference_and_named_connector() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let snapshot_mock = BindingSnapshotFixtures::service().await;
    snapshot_mock
        .mock_get_snapshot_background(
            project_id,
            binding,
            snapshot(project_id, binding, 1, &["connector.fetch"], None),
        )
        .await;
    let vault = VaultFixtures::service().await;
    vault
        .mock_kv_read("secret", "ops/token", "token", "injected-secret")
        .await;
    let connector_mock = rustycog::testing::wiremock::MockServerFixture::isolated().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
        .mount(&*connector_mock.server())
        .await;

    let descriptor = Arc::new(LazaretTestDescriptor);
    let fixture = TestFixture::new(descriptor).await.expect("fixture");
    let mut config = load_config().expect("config");
    config.manifesto_service.base_url = snapshot_mock.base_url();
    config.connectors = vec![ConnectorEntry {
        name: "docs".to_owned(),
        url: connector_mock.server().uri(),
    }];
    config.vault.base_url = vault.base_url();
    config.vault.token = "test-token".to_owned();
    config.vault.mount = "secret".to_owned();
    let app = Application::new(config).await.expect("app");
    let _fixture = fixture;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");
    let ok = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "connector.fetch",
            serde_json::json!({
                "connector": "docs",
                "path": "/hello",
                "secret": "secret:ops/token#token"
            }),
        ))
        .await;
    assert_eq!(ok.status_code(), 200, "{}", ok.text());
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_connector_body_too_large_is_413() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let snapshot_mock = BindingSnapshotFixtures::service().await;
    snapshot_mock
        .mock_get_snapshot_background(
            project_id,
            binding,
            snapshot(project_id, binding, 1, &["connector.fetch"], None),
        )
        .await;
    let connector_mock = rustycog::testing::wiremock::MockServerFixture::isolated().await;
    let huge = vec![b'x'; MAX_PAYLOAD_BYTES + 1];
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(huge))
        .mount(&*connector_mock.server())
        .await;
    let (_fixture, app) = boot(
        snapshot_mock.base_url(),
        vec![ConnectorEntry {
            name: "docs".to_owned(),
            url: connector_mock.server().uri(),
        }],
    )
    .await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");
    let resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "connector.fetch",
            serde_json::json!({"connector": "docs", "path": "/big"}),
        ))
        .await;
    assert_eq!(resp.status_code(), 413);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_invoke_kv_put_cas_zero_rejects_stale_cas() {
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
    let server = axum_test::TestServer::new(app.router()).expect("test server");

    let first = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1", "cas": 0}),
        ))
        .await;
    assert_eq!(first.status_code(), 200, "{}", first.text());

    let second = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.put",
            serde_json::json!({"key": "k", "value": "v1", "cas": 0}),
        ))
        .await;
    assert_eq!(second.status_code(), 400, "{}", second.text());
    let body: serde_json::Value = second.json();
    let error = body["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("cas mismatch"),
        "expected cas mismatch, got {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_invoke_empty_declared_is_capability_not_declared() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    let mut snap = snapshot(project_id, binding, 1, &["storage.kv.read"], None);
    snap.declared = vec![];
    mock.mock_get_snapshot_background(project_id, binding, snap)
        .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");
    let resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(resp.status_code(), 403, "{}", resp.text());
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"], "capability_not_declared");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_invoke_stale_consent_revision_is_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 5)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    let mut snap = snapshot(project_id, binding, 5, &["storage.kv.read"], None);
    snap.consents[0].grant_revision = 4;
    mock.mock_get_snapshot_background(project_id, binding, snap)
        .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");
    let resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(resp.status_code(), 403, "{}", resp.text());
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"], "consent_revision_mismatch");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t7_invoke_disabled_and_pending_component_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = WorkloadIdentity::try_new(Uuid::new_v4(), binding, "release-1".to_owned(), 1, 1)
        .expect("identity");
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        binding,
        snapshot(project_id, binding, 1, &["storage.kv.read"], None),
    )
    .await;

    let (_fixture, app) = boot(mock.base_url(), Vec::new()).await;
    let token = enroll_session(&app, project_id, identity).await;
    let server = axum_test::TestServer::new(app.router()).expect("test server");

    mock.reset().await;
    let mut disabled = snapshot(project_id, binding, 1, &["storage.kv.read"], None);
    disabled.component_status = "disabled".to_owned();
    mock.mock_get_snapshot_background(project_id, binding, disabled)
        .await;
    let disabled_resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(disabled_resp.status_code(), 403, "{}", disabled_resp.text());
    let disabled_body: serde_json::Value = disabled_resp.json();
    assert_eq!(disabled_body["error"], "component_inactive");

    mock.reset().await;
    let mut pending = snapshot(project_id, binding, 1, &["storage.kv.read"], None);
    pending.component_status = "pending".to_owned();
    mock.mock_get_snapshot_background(project_id, binding, pending)
        .await;
    let pending_resp = server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            binding,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(pending_resp.status_code(), 403, "{}", pending_resp.text());
    let pending_body: serde_json::Value = pending_resp.json();
    assert_eq!(pending_body["error"], "component_inactive");
}
