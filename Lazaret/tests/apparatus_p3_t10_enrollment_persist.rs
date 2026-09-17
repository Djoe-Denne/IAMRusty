//! Apparatus P3 — T10 enrollment persistence (Postgres) and revoke on `component_removed`.

mod common;

#[allow(unused)]
#[path = "fixtures/mod.rs"]
mod fixtures;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use apparatus_contracts::{ApparatusError, BindingId};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use common::setup_test_server;
use lazaret_application::{empty_command_registry, EnrollCommand, IdentityService, InvokeService};
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    AsyncKvStore, BindingGrantSnapshot, BindingGrantSnapshotPort, CapabilityConsent,
    ConnectorRegistry, EnrollmentStore, GrantFetchError, IdentityError, VerifiedClientCertificate,
    WorkloadIdentity,
};
use lazaret_http::create_router;
use lazaret_infra::{
    DedicatedSessionSigner, DeniedSecretResolver, KvPurgeEventHandler, NamedConnectorProxy,
    PlatformInternalCa, PostgresEnrollmentRegistry, PostgresKvStore,
};
use manifesto_events::{ComponentRemovedEvent, ManifestoDomainEvent};
use rcgen::{CertificateParams, KeyPair};
use rustycog::command::GenericCommandService;
use rustycog::events::{DomainEvent, EventHandler};
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use sea_orm::DatabaseConnection;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

struct MemoryPort {
    snapshots: HashMap<Uuid, BindingGrantSnapshot>,
}

#[async_trait]
impl BindingGrantSnapshotPort for MemoryPort {
    async fn fetch(
        &self,
        _project_id: Uuid,
        component_id: Uuid,
        _principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        self.snapshots
            .get(&component_id)
            .cloned()
            .ok_or(GrantFetchError::NotFound)
    }
}

struct ReplaceablePort {
    snapshot: Mutex<BindingGrantSnapshot>,
}

#[async_trait]
impl BindingGrantSnapshotPort for ReplaceablePort {
    async fn fetch(
        &self,
        _project_id: Uuid,
        _component_id: Uuid,
        _principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        Ok(self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone())
    }
}

struct NoopKv;

#[async_trait]
impl AsyncKvStore for NoopKv {
    async fn get(
        &self,
        _binding: &BindingId,
        _key: &str,
    ) -> Result<Option<Vec<u8>>, ApparatusError> {
        Ok(None)
    }

    async fn put(
        &self,
        _binding: &BindingId,
        _key: &str,
        _value: &[u8],
        _expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError> {
        Ok(1)
    }

    async fn delete(&self, _binding: &BindingId, _key: &str) -> Result<bool, ApparatusError> {
        Ok(false)
    }

    async fn purge(&self, _binding: &BindingId) -> Result<(), ApparatusError> {
        Ok(())
    }
}

struct RevokeFailsEnrollment;

#[async_trait]
impl EnrollmentStore for RevokeFailsEnrollment {
    async fn put(
        &self,
        _fingerprint: String,
        _identity: WorkloadIdentity,
    ) -> Result<(), IdentityError> {
        Ok(())
    }

    async fn get(&self, _fingerprint: &str) -> Option<WorkloadIdentity> {
        None
    }

    async fn binding_enrolled(&self, _binding: Uuid) -> Result<bool, IdentityError> {
        Ok(false)
    }

    async fn revoke_binding(&self, _binding: Uuid) -> Result<(), IdentityError> {
        Err(IdentityError::EnrollmentStore(
            "injected revoke failure".to_owned(),
        ))
    }
}

fn snapshot_for(identity: &WorkloadIdentity, project_id: Uuid) -> BindingGrantSnapshot {
    BindingGrantSnapshot {
        component_id: identity.binding,
        project_id,
        project_status: "active".to_owned(),
        component_status: "active".to_owned(),
        source: "managed".to_owned(),
        digest: Some(identity.release.clone()),
        desired_generation: identity.generation,
        observed_generation: identity.generation,
        grant_revision: identity.grant_revision,
        declared: vec!["storage.kv.read".to_owned()],
        consents: vec![CapabilityConsent {
            capability: "storage.kv.read".to_owned(),
            status: "consented".to_owned(),
            grant_revision: identity.grant_revision,
        }],
        principal: None,
    }
}

fn sample_identity() -> WorkloadIdentity {
    WorkloadIdentity::try_new(Uuid::new_v4(), Uuid::new_v4(), "release-1".to_owned(), 1, 0)
        .expect("identity")
}

fn port_for(identities: &[&WorkloadIdentity], project_id: Uuid) -> MemoryPort {
    MemoryPort {
        snapshots: identities
            .iter()
            .map(|identity| (identity.binding, snapshot_for(identity, project_id)))
            .collect(),
    }
}

fn workload_csr() -> String {
    let key = KeyPair::generate().expect("workload key stays local");
    let params = CertificateParams::new(vec!["workload.test".to_owned()]).expect("csr params");
    params
        .serialize_request(&key)
        .expect("csr")
        .pem()
        .expect("csr pem")
}

fn identity_service_on(
    db: DatabaseConnection,
    snapshots: Arc<dyn BindingGrantSnapshotPort>,
) -> Arc<IdentityService> {
    let config = IdentityConfig::default();
    Arc::new(IdentityService::new(
        Arc::new(PlatformInternalCa::new().expect("ca")),
        Arc::new(DedicatedSessionSigner::from_config(&config).expect("signer")),
        Arc::new(PostgresEnrollmentRegistry::new(db)),
        snapshots,
        config.session_ttl_minutes,
        config.cert_ttl_hours,
    ))
}

fn app_state() -> AppState {
    let command_registry = empty_command_registry();
    let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));
    let extractor = UserIdExtractor::from_resolved_secret("test-hs256-secret").expect("jwt");
    let checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
    AppState::new(command_service, extractor, checker)
}

fn invoke_stub(identity: Arc<IdentityService>) -> Arc<InvokeService> {
    let snapshots: Arc<dyn BindingGrantSnapshotPort> =
        Arc::new(port_for(&[&sample_identity()], Uuid::new_v4()));
    let grants = Arc::new(lazaret_application::GrantService::new(snapshots));
    let connectors =
        Arc::new(NamedConnectorProxy::new(ConnectorRegistry::default()).expect("connectors"));
    Arc::new(InvokeService::new(
        identity,
        grants,
        Arc::new(NoopKv),
        Arc::new(DeniedSecretResolver),
        connectors,
    ))
}

fn binding(id: Uuid) -> BindingId {
    id.to_string().parse().expect("binding")
}

fn removed(component_id: Uuid) -> Box<dyn DomainEvent> {
    ManifestoDomainEvent::ComponentRemoved(ComponentRemovedEvent::new(
        Uuid::new_v4(),
        component_id,
        "taskboard".to_owned(),
        Uuid::new_v4(),
        Utc::now(),
    ))
    .into()
}

async fn enroll(
    identity: &IdentityService,
    expected: WorkloadIdentity,
    project_id: Uuid,
) -> lazaret_domain::IssuedCertificate {
    identity
        .enroll(EnrollCommand {
            csr_pem: workload_csr(),
            identity: expected,
            project_id,
        })
        .await
        .expect("enroll")
}

async fn session_status(
    identity: Arc<IdentityService>,
    cert: VerifiedClientCertificate,
) -> StatusCode {
    let invoke = invoke_stub(identity.clone());
    let router = create_router(app_state(), identity, invoke);
    let mut request = Request::builder()
        .method("POST")
        .uri("/session")
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(cert);
    router.oneshot(request).await.expect("oneshot").status()
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_enrollment_survives_new_identity_service() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let db = fixture.db().as_ref().clone();
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(port_for(&[&expected], project_id));
    let identity_a = identity_service_on(db.clone(), snapshots.clone());
    let issued = enroll(&identity_a, expected.clone(), project_id).await;
    let cert = VerifiedClientCertificate::from_pem(&issued.pem).expect("cert");
    drop(identity_a);

    let identity_b = identity_service_on(db, snapshots);
    identity_b
        .issue_session(&cert)
        .await
        .expect("session after new process handle");
    assert_eq!(
        session_status(identity_b, cert).await,
        StatusCode::OK,
        "HTTP session must survive a new IdentityService on the same DB"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_component_removed_revokes_enrollment_and_purges_kv() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let db = fixture.db().as_ref().clone();
    let expected_a = sample_identity();
    let expected_b = sample_identity();
    let project_id = Uuid::new_v4();
    let snapshots: Arc<dyn BindingGrantSnapshotPort> =
        Arc::new(port_for(&[&expected_a, &expected_b], project_id));
    let identity = identity_service_on(db.clone(), snapshots);
    let issued_a = enroll(&identity, expected_a.clone(), project_id).await;
    let issued_b = enroll(&identity, expected_b.clone(), project_id).await;
    let cert_a = VerifiedClientCertificate::from_pem(&issued_a.pem).expect("cert a");
    let cert_b = VerifiedClientCertificate::from_pem(&issued_b.pem).expect("cert b");

    let kv: Arc<dyn AsyncKvStore> = Arc::new(PostgresKvStore::from_arc(&fixture.db()));
    let a = binding(expected_a.binding);
    let b = binding(expected_b.binding);
    kv.put(&a, "k", b"secret-a", None).await.expect("put a");
    kv.put(&b, "k", b"secret-b", None).await.expect("put b");

    let handler = KvPurgeEventHandler::new(kv.clone(), identity.enrollment_store());
    handler
        .handle_event(removed(expected_a.binding))
        .await
        .expect("revoke+purge a");

    let err = identity
        .issue_session(&cert_a)
        .await
        .expect_err("revoked fingerprint");
    assert!(
        matches!(err, IdentityError::NotEnrolled),
        "revoked session was {err:?}"
    );
    assert_eq!(
        session_status(identity.clone(), cert_a).await,
        StatusCode::UNAUTHORIZED
    );
    identity
        .issue_session(&cert_b)
        .await
        .expect("neighbor still enrolled");
    assert_eq!(
        session_status(identity.clone(), cert_b.clone()).await,
        StatusCode::OK
    );
    assert_eq!(kv.get(&a, "k").await.expect("purged a"), None);
    assert_eq!(
        kv.get(&b, "k").await.expect("b intact"),
        Some(b"secret-b".to_vec())
    );

    handler
        .handle_event(removed(expected_a.binding))
        .await
        .expect("second handle is a no-op");
    assert_eq!(kv.get(&a, "k").await.expect("a still gone"), None);
    assert_eq!(
        kv.get(&b, "k").await.expect("b still intact"),
        Some(b"secret-b".to_vec())
    );
    identity
        .issue_session(&cert_b)
        .await
        .expect("neighbor still enrolled after replay");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_second_enroll_same_binding_other_fingerprint_is_conflict() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let db = fixture.db().as_ref().clone();
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(port_for(&[&expected], project_id));
    let identity = identity_service_on(db, snapshots);
    let first = enroll(&identity, expected.clone(), project_id).await;
    let err = identity
        .enroll(EnrollCommand {
            csr_pem: workload_csr(),
            identity: expected.clone(),
            project_id,
        })
        .await
        .expect_err("second CSR");
    assert!(
        matches!(err, IdentityError::BindingAlreadyEnrolled),
        "second enroll was {err:?}"
    );

    let invoke = invoke_stub(identity.clone());
    let server =
        axum_test::TestServer::new(create_router(app_state(), identity.clone(), invoke.clone()))
            .expect("test server");
    let second = server
        .post("/enroll")
        .json(&serde_json::json!({
            "csr_pem": workload_csr(),
            "instance": expected.instance,
            "binding": expected.binding,
            "release": expected.release,
            "generation": expected.generation,
            "grant_revision": expected.grant_revision,
            "project_id": project_id,
        }))
        .await;
    assert_eq!(second.status_code(), 409, "{}", second.text());

    let cert = VerifiedClientCertificate::from_pem(&first.pem).expect("first cert");
    identity
        .issue_session(&cert)
        .await
        .expect("first cert still sessions");
    assert_eq!(session_status(identity, cert).await, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_revoke_failure_skips_kv_purge() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let kv: Arc<dyn AsyncKvStore> = Arc::new(PostgresKvStore::from_arc(&fixture.db()));
    let expected = sample_identity();
    let a = binding(expected.binding);
    kv.put(&a, "k", b"secret-a", None).await.expect("put a");

    let handler = KvPurgeEventHandler::new(kv.clone(), Arc::new(RevokeFailsEnrollment));
    handler
        .handle_event(removed(expected.binding))
        .await
        .expect_err("revoke failure must fail closed");
    assert_eq!(
        kv.get(&a, "k").await.expect("kv intact"),
        Some(b"secret-a".to_vec())
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_grant_revision_bump_no_reenroll_frozen_session() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let db = fixture.db().as_ref().clone();
    let expected = sample_identity();
    assert_eq!(expected.grant_revision, 0);
    let project_id = Uuid::new_v4();
    let port = Arc::new(ReplaceablePort {
        snapshot: Mutex::new(snapshot_for(&expected, project_id)),
    });
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = port.clone();
    let identity = identity_service_on(db, snapshots);
    let issued = enroll(&identity, expected.clone(), project_id).await;

    let mut bumped = snapshot_for(&expected, project_id);
    bumped.grant_revision = 1;
    *port
        .snapshot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = bumped;

    let err = identity
        .enroll(EnrollCommand {
            csr_pem: workload_csr(),
            identity: expected.clone(),
            project_id,
        })
        .await
        .expect_err("re-enroll after grant_revision bump");
    assert!(
        matches!(err, IdentityError::BindingAlreadyEnrolled),
        "re-enroll was {err:?}"
    );

    let invoke = invoke_stub(identity.clone());
    let server =
        axum_test::TestServer::new(create_router(app_state(), identity.clone(), invoke.clone()))
            .expect("test server");
    let second = server
        .post("/enroll")
        .json(&serde_json::json!({
            "csr_pem": workload_csr(),
            "instance": expected.instance,
            "binding": expected.binding,
            "release": expected.release,
            "generation": expected.generation,
            "grant_revision": 1,
            "project_id": project_id,
        }))
        .await;
    assert_eq!(second.status_code(), 409, "{}", second.text());

    let cert = VerifiedClientCertificate::from_pem(&issued.pem).expect("cert");
    let token = identity.issue_session(&cert).await.expect("session");
    let proof = identity.verify_session(&token).expect("verify");
    assert_eq!(proof.identity.grant_revision, 0);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_put_same_fingerprint_is_noop() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let db = fixture.db().as_ref().clone();
    let store = PostgresEnrollmentRegistry::new(db);
    let identity = sample_identity();
    let fingerprint = "a".repeat(64);
    store
        .put(fingerprint.clone(), identity.clone())
        .await
        .expect("first put");
    store
        .put(fingerprint.clone(), identity.clone())
        .await
        .expect("same fingerprint no-op");
    let got = store.get(&fingerprint).await.expect("row intact");
    assert_eq!(got, identity);
}
