//! Apparatus P3 — T3 identité workload (API typée + HTTP enroll/session).

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use apparatus_contracts::{ApparatusError, BindingId};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::create_jwt_token;
use fixtures::BindingSnapshotFixtures;
use jsonwebtoken::decode_header;
use lazaret_application::{empty_command_registry, EnrollCommand, IdentityService, InvokeService};
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    authorization_from_session, AsyncKvStore, AuthorizationDecision, BindingGrantSnapshot,
    BindingGrantSnapshotPort, CapabilityConsent, ConnectorRegistry, GrantFetchError, IdentityError,
    VerifiedClientCertificate, WorkloadIdentity, SESSION_AUDIENCE, SESSION_ISSUER,
};
use lazaret_http::create_router;
use lazaret_infra::{
    DedicatedSessionSigner, DeniedSecretResolver, HttpBindingGrantClient,
    InMemoryEnrollmentRegistry, NamedConnectorProxy, PlatformInternalCa,
};
use rcgen::{CertificateParams, CustomExtension, KeyPair};
use rustycog::command::GenericCommandService;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use tower::ServiceExt;
use uuid::Uuid;

struct MemoryPort {
    snapshot: BindingGrantSnapshot,
}

#[async_trait]
impl BindingGrantSnapshotPort for MemoryPort {
    async fn fetch(
        &self,
        _project_id: Uuid,
        _component_id: Uuid,
        _principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        Ok(self.snapshot.clone())
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

fn identity_service_from_snapshots(
    snapshots: Arc<dyn BindingGrantSnapshotPort>,
) -> Arc<IdentityService> {
    let config = IdentityConfig::default();
    Arc::new(IdentityService::new(
        Arc::new(PlatformInternalCa::new().expect("ca")),
        Arc::new(DedicatedSessionSigner::from_config(&config).expect("signer")),
        Arc::new(InMemoryEnrollmentRegistry::new()),
        snapshots,
        config.session_ttl_minutes,
        config.cert_ttl_hours,
    ))
}

fn identity_service_for(identity: &WorkloadIdentity, project_id: Uuid) -> Arc<IdentityService> {
    identity_service_from_snapshots(Arc::new(MemoryPort {
        snapshot: snapshot_for(identity, project_id),
    }))
}

fn identity_service() -> Arc<IdentityService> {
    identity_service_for(&sample_identity(), Uuid::new_v4())
}

fn sample_identity() -> WorkloadIdentity {
    WorkloadIdentity::try_new(Uuid::new_v4(), Uuid::new_v4(), "release-1".to_owned(), 1, 0)
        .expect("identity")
}

fn workload_csr() -> (String, String) {
    let key = KeyPair::generate().expect("workload key stays local");
    let params = CertificateParams::new(vec!["workload.test".to_owned()]).expect("csr params");
    let csr = params.serialize_request(&key).expect("csr");
    (csr.pem().expect("csr pem"), key.serialize_pem())
}

fn ca_csr_pem() -> String {
    let key = KeyPair::generate().expect("key");
    let mut params = CertificateParams::new(vec!["ca.workload.test".to_owned()]).expect("params");
    params
        .custom_extensions
        .push(CustomExtension::from_oid_content(
            &[2, 5, 29, 19],
            vec![0x30, 0x03, 0x01, 0x01, 0xff],
        ));
    params
        .serialize_request(&key)
        .expect("csr")
        .pem()
        .expect("pem")
}

fn app_state() -> AppState {
    let command_registry = empty_command_registry();
    let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));
    let extractor = UserIdExtractor::from_resolved_secret("test-hs256-secret").expect("jwt");
    let checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
    AppState::new(command_service, extractor, checker)
}

fn invoke_stub(identity: Arc<IdentityService>) -> Arc<InvokeService> {
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(MemoryPort {
        snapshot: snapshot_for(&sample_identity(), Uuid::new_v4()),
    });
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

#[test]
fn t3_iam_jwt_is_rejected_by_session_verify() {
    let identity = identity_service();
    let iam = create_jwt_token(Uuid::new_v4());
    let header = decode_header(&iam).expect("IAM header");
    assert_eq!(header.alg, jsonwebtoken::Algorithm::HS256);
    let err = identity
        .verify_session(&iam)
        .expect_err("IAM JWT must not verify as a Lazaret session");
    assert!(
        matches!(
            err,
            IdentityError::InvalidSession | IdentityError::UserClaimForbidden
        ),
        "IAM rejection was {err:?}"
    );
}

#[tokio::test]
async fn t3_session_without_enrollment_is_refused() {
    let identity = identity_service();
    let stray = VerifiedClientCertificate::from_der(vec![0x30, 0x00, 0x01, 0x02]);
    let err = identity
        .issue_session(&stray)
        .await
        .expect_err("session without enrollment");
    assert!(matches!(err, IdentityError::NotEnrolled));
}

#[tokio::test]
async fn t3_enroll_csr_then_verified_cert_then_session_claims() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let identity = identity_service_for(&expected, project_id);
    let (csr_pem, private_pem) = workload_csr();
    assert!(
        private_pem.to_ascii_uppercase().contains("PRIVATE KEY"),
        "workload must hold a private key locally"
    );
    assert!(
        !csr_pem.to_ascii_uppercase().contains("PRIVATE KEY"),
        "CSR must not embed the private key"
    );

    let issued = identity
        .enroll(EnrollCommand {
            csr_pem: csr_pem.clone(),
            identity: expected.clone(),
            project_id,
        })
        .await
        .expect("enroll");
    assert!(
        issued.pem.contains("BEGIN CERTIFICATE"),
        "CA must return a certificate PEM"
    );
    assert!(
        !issued.pem.to_ascii_uppercase().contains("PRIVATE KEY"),
        "CA must not return a private key"
    );

    let cert = VerifiedClientCertificate::from_pem(&issued.pem).expect("verified cert");
    let token = identity.issue_session(&cert).await.expect("session");

    let header = decode_header(&token).expect("session header");
    assert_ne!(
        header.alg,
        jsonwebtoken::Algorithm::HS256,
        "prefer EdDSA; HS256 IAM algorithm must not be the session alg"
    );
    assert_eq!(header.alg, jsonwebtoken::Algorithm::EdDSA);

    let proof = identity.verify_session(&token).expect("verify session");
    assert_eq!(proof.identity, expected);
    assert_eq!(proof.audience, SESSION_AUDIENCE);
    assert_eq!(proof.issuer, SESSION_ISSUER);
    assert!(proof.expires_at_unix > 0);
    assert_eq!(proof.identity.generation, 1);
    assert_eq!(proof.identity.grant_revision, 0);

    let payload = token.split('.').nth(1).expect("payload");
    let decoded = base64_decode(payload);
    let value: serde_json::Value = serde_json::from_slice(&decoded).expect("json");
    assert!(value.get("sub").is_none(), "no user sub claim");
    assert_eq!(value["aud"], SESSION_AUDIENCE);
    assert_eq!(value["iss"], SESSION_ISSUER);
    assert_eq!(value["instance"], expected.instance.to_string());
    assert_eq!(value["binding"], expected.binding.to_string());
    assert_eq!(value["release"], expected.release);
    assert_eq!(value["generation"], expected.generation);
    assert_eq!(value["grant_revision"], expected.grant_revision);
    assert!(value.get("exp").is_some());

    let decision = identity.authorization_from_session(&proof);
    assert_eq!(decision, AuthorizationDecision::LiveManifestoCheckRequired);
    assert_eq!(
        authorization_from_session(&proof),
        AuthorizationDecision::LiveManifestoCheckRequired
    );
}

#[tokio::test]
async fn t3_enroll_rewrites_generation_from_snapshot() {
    let mut body = sample_identity();
    body.generation = 9;
    body.grant_revision = 4;
    let project_id = Uuid::new_v4();
    let live = WorkloadIdentity::try_new(body.instance, body.binding, body.release.clone(), 1, 2)
        .expect("live");
    let identity = identity_service_for(&live, project_id);
    let (csr_pem, _) = workload_csr();
    let issued = identity
        .enroll(EnrollCommand {
            csr_pem,
            identity: body,
            project_id,
        })
        .await
        .expect("enroll");
    let cert = VerifiedClientCertificate::from_pem(&issued.pem).expect("cert");
    let token = identity.issue_session(&cert).await.expect("session");
    let proof = identity.verify_session(&token).expect("verify");
    assert_eq!(proof.identity.generation, 1);
    assert_eq!(proof.identity.grant_revision, 2);
}

#[tokio::test]
async fn t3_private_key_never_accepted_on_enroll() {
    let identity = identity_service();
    let (_csr_pem, private_pem) = workload_csr();
    let err = identity
        .enroll(EnrollCommand {
            csr_pem: private_pem,
            identity: sample_identity(),
            project_id: Uuid::new_v4(),
        })
        .await
        .expect_err("private key must be rejected");
    assert!(matches!(err, IdentityError::PrivateKeyNotAccepted));
}

#[tokio::test]
async fn t3_csr_isca_rejected() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let identity = identity_service_for(&expected, project_id);
    let err = identity
        .enroll(EnrollCommand {
            csr_pem: ca_csr_pem(),
            identity: expected,
            project_id,
        })
        .await
        .expect_err("CA CSR must be rejected");
    assert!(
        matches!(err, IdentityError::InvalidCsr(_)),
        "CA CSR rejection was {err:?}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn t3_http_enroll_consults_snapshot() {
    let project_id = Uuid::new_v4();
    let expected = sample_identity();
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        expected.binding,
        snapshot_for(&expected, project_id),
    )
    .await;
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(
        HttpBindingGrantClient::from_config(&lazaret_configuration::ManifestoServiceConfig {
            base_url: mock.base_url(),
            timeout_seconds: 5,
            hs256_secret: Some("manifesto-grant-snapshot-test-hs256".to_owned()),
            ..lazaret_configuration::ManifestoServiceConfig::default()
        })
        .expect("client"),
    );
    let identity = identity_service_from_snapshots(snapshots);
    let invoke = invoke_stub(identity.clone());
    let server = axum_test::TestServer::new(create_router(app_state(), identity, invoke))
        .expect("test server");
    let (csr_pem, _) = workload_csr();
    let resp = server
        .post("/enroll")
        .json(&serde_json::json!({
            "csr_pem": csr_pem,
            "instance": expected.instance,
            "binding": expected.binding,
            "release": expected.release,
            "generation": expected.generation,
            "grant_revision": expected.grant_revision,
            "project_id": project_id,
        }))
        .await;
    assert_eq!(resp.status_code(), 200, "{}", resp.text());
    let requests = mock.received_requests().await;
    let gets: Vec<_> = requests
        .iter()
        .filter(|req| req.method.as_str() == "GET")
        .collect();
    assert!(
        !gets.is_empty(),
        "enroll must consult snapshot with at least one GET, got {} requests",
        requests.len()
    );
    for req in gets {
        let query = req.url.query().unwrap_or("");
        assert!(
            !query.contains("principal"),
            "snapshot GET must omit principal query, got {query}"
        );
    }
    let body: serde_json::Value = resp.json();
    assert!(body["certificate_pem"]
        .as_str()
        .expect("pem")
        .contains("BEGIN CERTIFICATE"));
}

#[tokio::test]
async fn t3_http_session_without_cert_is_401() {
    let identity = identity_service();
    let invoke = invoke_stub(identity.clone());
    let server = axum_test::TestServer::new(create_router(app_state(), identity, invoke))
        .expect("test server");
    let resp = server.post("/session").await;
    assert_eq!(resp.status_code(), 401);
}

#[tokio::test]
async fn t3_http_session_with_enrolled_cert_is_200() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let identity = identity_service_for(&expected, project_id);
    let (csr_pem, _) = workload_csr();
    let issued = identity
        .enroll(EnrollCommand {
            csr_pem,
            identity: expected,
            project_id,
        })
        .await
        .expect("enroll");
    let cert = VerifiedClientCertificate::from_pem(&issued.pem).expect("cert");
    let invoke = invoke_stub(identity.clone());
    let router = create_router(app_state(), identity, invoke);
    let mut request = Request::builder()
        .method("POST")
        .uri("/session")
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(cert);
    let response = router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial_test::serial]
async fn t3_http_second_csr_same_binding_is_409_first_cert_still_sessions() {
    let project_id = Uuid::new_v4();
    let expected = sample_identity();
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(
        project_id,
        expected.binding,
        snapshot_for(&expected, project_id),
    )
    .await;
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(
        HttpBindingGrantClient::from_config(&lazaret_configuration::ManifestoServiceConfig {
            base_url: mock.base_url(),
            timeout_seconds: 5,
            hs256_secret: Some("manifesto-grant-snapshot-test-hs256".to_owned()),
            ..lazaret_configuration::ManifestoServiceConfig::default()
        })
        .expect("client"),
    );
    let identity = identity_service_from_snapshots(snapshots);
    let invoke = invoke_stub(identity.clone());
    let server =
        axum_test::TestServer::new(create_router(app_state(), identity.clone(), invoke.clone()))
            .expect("test server");

    let (csr_a, _) = workload_csr();
    let first = server
        .post("/enroll")
        .json(&serde_json::json!({
            "csr_pem": csr_a,
            "instance": expected.instance,
            "binding": expected.binding,
            "release": expected.release.clone(),
            "generation": expected.generation,
            "grant_revision": expected.grant_revision,
            "project_id": project_id,
        }))
        .await;
    assert_eq!(first.status_code(), 200, "{}", first.text());
    let first_body: serde_json::Value = first.json();
    let pem = first_body["certificate_pem"]
        .as_str()
        .expect("certificate_pem")
        .to_owned();

    let (csr_b, _) = workload_csr();
    let second = server
        .post("/enroll")
        .json(&serde_json::json!({
            "csr_pem": csr_b,
            "instance": expected.instance,
            "binding": expected.binding,
            "release": expected.release.clone(),
            "generation": expected.generation,
            "grant_revision": expected.grant_revision,
            "project_id": project_id,
        }))
        .await;
    assert_eq!(second.status_code(), 409, "{}", second.text());

    let cert = VerifiedClientCertificate::from_pem(&pem).expect("first cert");
    let router = create_router(app_state(), identity, invoke);
    let mut request = Request::builder()
        .method("POST")
        .uri("/session")
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(cert);
    let response = router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
}

fn base64_decode(input: &str) -> Vec<u8> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(input))
        .expect("b64")
}
