//! G-1/G-8/G-10: session dispatch using the EXISTING t11b TLS harness.
//! SQL runtime store and real TLS to a WireMock-backed C collaborator.
//! ApprovedInventory arranges dispatch readiness ONLY, not maintenance authority
//! or proof of a distributed cutover. Legacy helpers are the historical t11b seed.

use super::{
    app_state, create_prefixed_router, https_client, identity_service, install_crypto, invoke_stub,
    jwt_payload, runtime_control_tls, serve_live_router, workload_csr, LiveServer,
};
use apparatus_contracts::runtime::{InstallRuntimeLegacyFenceV1, RuntimeDigestV1};
use async_trait::async_trait;
use lazaret_application::{
    runtime::{RuntimeIdentityAuthority, RuntimeSessionService},
    runtime_cutover::{RuntimeBootstrapMaintenance, RuntimeCutoverGuard},
    EnrollCommand, IdentityService,
};
use lazaret_domain::{
    runtime::{RuntimeEnrollment, RuntimeError, RuntimeStore},
    CertificateAuthority, VerifiedClientCertificate, WorkloadIdentity,
};
use lazaret_infra::{
    runtime_control::HttpRuntimeControl, runtime_store::PostgresRuntimeStore, PlatformInternalCa,
};
use readiness::ReadinessProbe;
use rustycog::testing::{wiremock::MockServerFixture, TestFixture};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path, query_param},
    Mock, ResponseTemplate,
};

struct ApprovedInventory {
    project: Uuid,
}

#[async_trait]
impl RuntimeBootstrapMaintenance for ApprovedInventory {
    async fn verify_inventory(
        &self,
        projects: &[Uuid],
        inventory: &[InstallRuntimeLegacyFenceV1],
    ) -> Result<(), RuntimeError> {
        assert_eq!(projects, [self.project], "G-8 explicit fixture scope");
        assert!(
            inventory.is_empty(),
            "G-8 bootstrap fixture starts before any materialization"
        );
        Ok(())
    }

    async fn verify_before_open(&self) -> Result<(), RuntimeError> {
        Ok(())
    }
}

struct Dispatch {
    _fixture: TestFixture,
    _relay: LiveServer,
    app: LiveServer,
    backend: MockServerFixture,
    identity: Arc<IdentityService>,
    store: Arc<PostgresRuntimeStore>,
    guard: Arc<RuntimeCutoverGuard>,
    project: Uuid,
    workload: WorkloadIdentity,
}

impl Dispatch {
    async fn start() -> Self {
        install_crypto();
        let fixture = TestFixture::new(Arc::new(super::common::LazaretTestDescriptor))
            .await
            .expect("G-8 real SQL fixture");
        let backend = MockServerFixture::isolated().await;
        let relay_ca = Arc::new(PlatformInternalCa::new().expect("relay client CA"));
        let relay = serve_live_router(
            runtime_control_tls::wiremock_proxy_router(backend.server().uri()),
            relay_ca.clone(),
            true,
        )
        .await;
        let (csr, key) = workload_csr();
        let cert = relay_ca
            .sign_csr(&csr, 1, Uuid::new_v4())
            .expect("real service TLS client cert");
        let client_identity = reqwest::Identity::from_pem(format!("{}{key}", cert.pem).as_bytes())
            .expect("relay client identity");
        let control = Arc::new(
            HttpRuntimeControl::new(&relay.base, client_identity, relay.server_ca.clone())
                .expect("C HTTP adapter with real TLS"),
        );
        let store = Arc::new(PostgresRuntimeStore::new(fixture.db()));
        let guard = Arc::new(RuntimeCutoverGuard::new(store.clone(), control.clone()));
        let project = Uuid::new_v4();
        Mock::given(method("GET"))
            .and(path("/internal/apparatus/runtime/bindings"))
            .and(query_param("project_id", project.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "schema_version": 1, "items": [], "next_cursor": null
            })))
            .mount(&backend.server())
            .await;
        guard
            .reconcile(&ApprovedInventory { project }, &[project], &[])
            .await
            .expect("dispatch-only fixture bootstrap");
        assert!(guard.is_ready());
        backend.server().reset().await;
        let ca = Arc::new(PlatformInternalCa::new().expect("workload CA"));
        let workload = WorkloadIdentity::try_new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            format!("sha256:{}", "b".repeat(64)),
            1,
            0,
        )
        .expect("workload");
        let identity = identity_service(ca.clone(), &workload, project);
        let router = create_prefixed_router(
            app_state(),
            Arc::new(ReadinessProbe::new("lazaret")),
            identity.clone(),
            invoke_stub(identity.clone()),
        );
        let sessions = Arc::new(RuntimeSessionService::new(
            identity.clone(),
            store.clone(),
            Arc::new(RuntimeIdentityAuthority::new(control)),
            guard.clone(),
        ));
        let router = lazaret_http::with_runtime_session(router, sessions);
        let app = serve_live_router(router, ca, false).await;
        Self {
            _fixture: fixture,
            _relay: relay,
            app,
            backend,
            identity,
            store,
            guard,
            project,
            workload,
        }
    }

    async fn legacy_client(&self) -> Vec<u8> {
        let (csr_pem, key) = workload_csr();
        let cert = self
            .identity
            .enroll(EnrollCommand {
                csr_pem,
                identity: self.workload.clone(),
                project_id: self.project,
            })
            .await
            .expect("historical IdentityService credential seed");
        format!("{}{key}", cert.pem).into_bytes()
    }

    async fn install_fence(&self, project: Uuid) {
        self.store
            .install_legacy_fence(&InstallRuntimeLegacyFenceV1 {
                schema_version: 1,
                migration_id: Uuid::new_v4(),
                project_id: project,
                binding_id: self.workload.binding,
                inventory_sha256: RuntimeDigestV1::new(format!("sha256:{}", "d".repeat(64)))
                    .expect("inventory digest"),
            })
            .await
            .expect("durable local negative fence");
    }

    async fn runtime_client(&self) -> (Vec<u8>, RuntimeEnrollment) {
        let (csr, key) = workload_csr();
        let cert = self
            .app
            .ca
            .sign_runtime_csr(&csr, 180, self.workload.binding)
            .expect("real runtime client certificate");
        let verified = VerifiedClientCertificate::from_pem(&cert.pem).expect("certificate");
        let now = chrono::Utc::now().timestamp_millis();
        let record = RuntimeEnrollment {
            project_id: self.project,
            identity: self.workload.clone(),
            // Testcontainers shares this database across serial tests; Pod UID
            // uniqueness is durable even after a prior test closes its row.
            pod_uid: format!("dispatch-runtime-pod-{}", Uuid::new_v4()),
            revision_uid: "dispatch-runtime-revision".to_owned(),
            namespace: "apparatus-plugins".to_owned(),
            service_account: "apparatus-runtime".to_owned(),
            service_account_uid: "dispatch-runtime-service-account".to_owned(),
            revocation_epoch: 2,
            // RuntimeEnrollment's SQL field stores bare SHA-256 hex, unlike
            // RuntimeDigestV1 wire values which include the sha256: prefix.
            csr_sha256: apparatus_contracts::digest_str(&csr)
                .as_str()
                .strip_prefix("sha256:")
                .expect("CSR digest prefix")
                .to_owned(),
            fingerprint: verified.fingerprint_sha256(),
            certificate_pem: cert.pem.clone(),
            issued_at_unix_ms: now,
            expires_at_unix_ms: now + 120_000,
            closed: false,
        };
        self.store
            .enroll(&record)
            .await
            .expect("real SQL runtime enrollment arrange");
        (format!("{}{key}", cert.pem).into_bytes(), record)
    }

    async fn session(&self, client: &[u8]) -> (u16, serde_json::Value) {
        let response = https_client(Some(client))
            .post(format!("{}/lazaret/session", self.app.base))
            .send()
            .await
            .expect("real HTTPS session");
        let status = response.status().as_u16();
        let body = response.json().await.expect("session envelope");
        (status, body)
    }

    async fn request_count(&self) -> usize {
        self.backend
            .server()
            .received_requests()
            .await
            .expect("C request log")
            .len()
    }

    async fn mock_instance_snapshot(
        &self,
        record: &RuntimeEnrollment,
        snapshot: serde_json::Value,
    ) {
        self.backend.server().reset().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/internal/apparatus/runtime/bindings/{}",
                record.identity.binding
            )))
            .and(query_param("project_id", record.project_id.to_string()))
            .and(query_param("instances_mode", "by_instance_id"))
            .and(query_param(
                "instance_id",
                record.identity.instance.to_string(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(snapshot))
            .expect(1)
            .mount(&self.backend.server())
            .await;
    }
}

fn current_snapshot(record: &RuntimeEnrollment) -> serde_json::Value {
    let now = chrono::Utc::now().timestamp_millis();
    let cas = serde_json::json!({
        "generation": record.identity.generation.to_string(),
        "revocation_epoch": record.revocation_epoch.to_string(),
        "projection_version": "13", "grant_revision": record.identity.grant_revision.to_string()
    });
    serde_json::json!({
        "schema_version": 1, "project_id": record.project_id,
        "binding_id": record.identity.binding, "cas": cas, "state": "active",
        "release": record.identity.release, "profile": "knative-v1", "observation": null,
        "instances": {
            "schema_version": 1, "next_cursor": null, "read_at_unix_ms": now.to_string(),
            "items": [{
                "schema_version": 1, "project_id": record.project_id,
                "binding_id": record.identity.binding, "instance_id": record.identity.instance,
                "kind": "pod", "state": "current", "cas": cas, "release": record.identity.release,
                "pod": { "pod_uid": record.pod_uid, "revision_uid": record.revision_uid,
                    "namespace": record.namespace, "service_account": record.service_account,
                    "service_account_uid": record.service_account_uid },
                "writer_fence": "15", "observed_at_unix_ms": now.to_string(),
                "valid_until_unix_ms": (now + 5000).to_string(), "closed_at_unix_ms": null
            }]
        }
    })
}

#[tokio::test]
#[serial]
async fn g1_g8_legacy_nonmigrated_session_keeps_claims_and_never_calls_runtime_writer() {
    let h = Dispatch::start().await;
    let client = h.legacy_client().await;
    let (status, body) = h.session(&client).await;
    assert_eq!(status, 200, "G-1 legacy nonmigrated: {body}");
    let claims = jwt_payload(body["session_token"].as_str().expect("legacy session"));
    assert_eq!(claims["binding"], h.workload.binding.to_string());
    assert_eq!(claims["instance"], h.workload.instance.to_string());
    assert!(
        claims.get("project_id").is_none(),
        "G-1 legacy claims unchanged"
    );
    assert_eq!(
        h.request_count().await,
        0,
        "G-1 no RuntimeControl call for legacy"
    );
}

#[tokio::test]
#[serial]
async fn g1_g8_legacy_fenced_before_c_prepare_has_no_session_no_runtime_lookup_or_fallback() {
    let h = Dispatch::start().await;
    let client = h.legacy_client().await;
    h.install_fence(h.project).await;
    let (status, body) = h.session(&client).await;
    assert_eq!(status, 403, "G-8 fence before C materialization: {body}");
    assert_eq!(body["error"], "runtime_legacy_disabled");
    assert!(
        body.get("session_token").is_none(),
        "G-8 must not sign a session"
    );
    assert_eq!(
        h.request_count().await,
        0,
        "G-8 no missing C row can remove D refusal"
    );
    assert!(h
        .store
        .cutover_fence(h.workload.binding)
        .await
        .expect("fence read")
        .is_some());
}

#[tokio::test]
#[serial]
async fn g1_g8_bootstrap_closed_refuses_otherwise_valid_legacy_certificate() {
    let h = Dispatch::start().await;
    let client = h.legacy_client().await;
    h.guard.close();
    let (status, body) = h.session(&client).await;
    assert_eq!(status, 502, "G-8 closed bootstrap: {body}");
    assert_eq!(body["error"], "runtime_projection_unavailable");
    assert!(body.get("session_token").is_none());
    assert_eq!(h.request_count().await, 0);
}

#[tokio::test]
#[serial]
async fn g1_g8_runtime_missing_fence_denies_and_wrong_project_install_refuses_without_mutation() {
    let h = Dispatch::start().await;
    // Deliberately inconsistent historical seed, ONLY for this negative case.
    // Successful dispatch cases install the fence before issuing a fresh identity.
    let (client, record) = h.runtime_client().await;
    let (status, body) = h.session(&client).await;
    assert_eq!(status, 502, "G-8 missing local mapping: {body}");
    assert_eq!(body["error"], "runtime_projection_unavailable");
    let result = h
        .store
        .install_legacy_fence(&InstallRuntimeLegacyFenceV1 {
            schema_version: 1,
            migration_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            binding_id: h.workload.binding,
            inventory_sha256: RuntimeDigestV1::new(format!("sha256:{}", "d".repeat(64)))
                .expect("inventory digest"),
        })
        .await;
    assert!(
        matches!(result, Err(RuntimeError::RuntimeIdReused)),
        "G-8 cross-project install must conflict"
    );
    assert!(h
        .store
        .cutover_fence(h.workload.binding)
        .await
        .expect("negative fence read")
        .is_none());
    let after = h
        .store
        .enrollment_by_instance(record.identity.instance)
        .await
        .expect("runtime enrollment read")
        .expect("unchanged enrollment");
    assert_eq!(
        after.project_id, record.project_id,
        "G-8 install must not reassign project"
    );
    assert_eq!(
        after.fingerprint, record.fingerprint,
        "G-8 install must not reassign credential"
    );
    assert!(
        !after.closed,
        "G-8 failed install must not close the existing identity"
    );
    let (status, body) = h.session(&client).await;
    assert_eq!(
        status, 502,
        "G-8 failed install leaves missing fence denied: {body}"
    );
    assert_eq!(body["error"], "runtime_projection_unavailable");
    assert!(body.get("session_token").is_none());
    assert_eq!(h.request_count().await, 0);
}

#[tokio::test]
#[serial]
async fn g1_g8_runtime_session_reads_store_project_and_exact_instance_every_time() {
    let h = Dispatch::start().await;
    h.install_fence(h.project).await;
    let (client, record) = h.runtime_client().await;
    h.mock_instance_snapshot(&record, current_snapshot(&record))
        .await;
    let (status, body) = h.session(&client).await;
    assert_eq!(
        status, 200,
        "G-1 current authoritative runtime identity: {body}"
    );
    let claims = jwt_payload(body["session_token"].as_str().expect("runtime session"));
    assert_eq!(claims["instance"], record.identity.instance.to_string());
    assert_eq!(claims["binding"], record.identity.binding.to_string());
    assert_eq!(
        h.request_count().await,
        1,
        "G-1 exact instance lookup over real mTLS"
    );
    let mut revoked = current_snapshot(&record);
    revoked["instances"]["items"][0]["state"] = serde_json::json!("tombstone");
    revoked["instances"]["items"][0]["closed_at_unix_ms"] =
        serde_json::json!(chrono::Utc::now().timestamp_millis().to_string());
    h.mock_instance_snapshot(&record, revoked).await;
    let (status, body) = h.session(&client).await;
    assert_eq!(
        status, 403,
        "G-1 same crypto-valid cert after writer revoke: {body}"
    );
    assert_eq!(body["error"], "runtime_identity_revoked");
    assert!(body.get("session_token").is_none());
    assert_eq!(
        h.request_count().await,
        1,
        "G-1 revalidation cannot use cached allow"
    );
}

#[tokio::test]
#[serial]
async fn g1_g8_runtime_registry_incomplete_multimatch_or_stale_never_chooses_first_or_allows() {
    let h = Dispatch::start().await;
    h.install_fence(h.project).await;
    let (client, record) = h.runtime_client().await;
    for case in ["null", "omitted", "multiple", "stale", "wrong_project"] {
        let mut snapshot = current_snapshot(&record);
        let expected_error = match case {
            "null" => {
                snapshot["instances"] = serde_json::Value::Null;
                "runtime_authority_protocol_error"
            }
            "omitted" => {
                snapshot
                    .as_object_mut()
                    .expect("binding snapshot")
                    .remove("instances");
                "runtime_authority_protocol_error"
            }
            "multiple" => {
                let duplicate = snapshot["instances"]["items"][0].clone();
                snapshot["instances"]["items"]
                    .as_array_mut()
                    .expect("instance list")
                    .push(duplicate);
                "runtime_authority_protocol_error"
            }
            "stale" => {
                let expired = chrono::Utc::now().timestamp_millis() - 1;
                snapshot["instances"]["items"][0]["observed_at_unix_ms"] =
                    serde_json::json!((expired - 5000).to_string());
                snapshot["instances"]["items"][0]["valid_until_unix_ms"] =
                    serde_json::json!(expired.to_string());
                "runtime_projection_unavailable"
            }
            "wrong_project" => {
                snapshot["project_id"] = serde_json::json!(Uuid::new_v4());
                "runtime_authority_protocol_error"
            }
            _ => unreachable!("finite fixture cases"),
        };
        h.mock_instance_snapshot(&record, snapshot).await;
        let (status, body) = h.session(&client).await;
        assert_eq!(status, 502, "G-1 {case}: {body}");
        assert_eq!(body["error"], expected_error, "G-1 {case}");
        assert!(body.get("session_token").is_none(), "G-1 {case}");
        assert_eq!(
            h.request_count().await,
            1,
            "G-1 no duplicate authority read/retry for {case}"
        );
    }
}

#[tokio::test]
#[serial]
async fn g1_g8_fingerprint_present_in_both_stores_is_refused_before_authority_lookup() {
    let h = Dispatch::start().await;
    h.install_fence(h.project).await;
    let (client, record) = h.runtime_client().await;
    h.identity
        .enrollment_store()
        .put(record.fingerprint.clone(), record.identity.clone())
        .await
        .expect("arrange contradictory historical enrollment");
    let (status, body) = h.session(&client).await;
    assert_eq!(status, 401, "G-1 ambiguous fingerprint: {body}");
    assert_eq!(body["error"], "unauthorized");
    assert!(body.get("session_token").is_none());
    assert_eq!(h.request_count().await, 0);
}

#[tokio::test]
#[serial]
async fn g1_g8_closed_runtime_enrollment_never_uses_available_legacy_path() {
    let h = Dispatch::start().await;
    h.install_fence(h.project).await;
    let (client, _) = h.runtime_client().await;
    h.store
        .close_binding(h.project, h.workload.binding)
        .await
        .expect("close SQL runtime enrollment");
    let (status, body) = h.session(&client).await;
    assert_eq!(status, 403, "G-1 closed runtime enrollment: {body}");
    assert_eq!(body["error"], "runtime_identity_revoked");
    assert!(body.get("session_token").is_none());
    assert_eq!(h.request_count().await, 0);
}

#[tokio::test]
#[serial]
async fn g1_g8_runtime_sql_lookup_failure_is_not_absence_and_cannot_issue_legacy_session() {
    let h = Dispatch::start().await;
    let client = h.legacy_client().await;
    let db = h._fixture.db();
    // Owned test database only. Restore the table BEFORE inspecting the HTTP
    // result, including when transport fails, to avoid poisoning later fixtures.
    db.execute(Statement::from_string(DatabaseBackend::Postgres,
        "ALTER TABLE apparatus_runtime_enrollments RENAME TO apparatus_runtime_enrollments_g1_unavailable"
    )).await.expect("arrange real SQL read failure");
    let response = https_client(Some(&client))
        .post(format!("{}/lazaret/session", h.app.base))
        .send()
        .await;
    db.execute(Statement::from_string(DatabaseBackend::Postgres,
        "ALTER TABLE apparatus_runtime_enrollments_g1_unavailable RENAME TO apparatus_runtime_enrollments"
    )).await.expect("restore owned runtime table");
    let response = response.expect("G-1 session response during SQL failure");
    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await.expect("runtime SQL error envelope");
    assert_eq!(
        status, 502,
        "G-1 SQL unavailable must not become legacy absent: {body}"
    );
    assert_eq!(body["error"], "runtime_authority_unavailable");
    assert!(body.get("session_token").is_none());
    assert_eq!(h.request_count().await, 0);
    let (status, body) = h.session(&client).await;
    assert_eq!(
        status, 200,
        "G-1 positive control after real SQL recovery: {body}"
    );
}

#[tokio::test]
#[serial]
async fn g1_g8_incomplete_bootstrap_reconciliation_closes_previously_ready_session_dispatch() {
    let h = Dispatch::start().await;
    let client = h.legacy_client().await;
    let mut unaccounted_binding = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../crates/apparatus-contracts/tests/fixtures/runtime_shapes_v1.json"
    ))
    .expect("binding wire fixture")["binding_empty_instances"]
        .clone();
    unaccounted_binding["project_id"] = serde_json::json!(h.project);
    unaccounted_binding["instances"] = serde_json::Value::Null;
    Mock::given(method("GET"))
        .and(path("/internal/apparatus/runtime/bindings"))
        .and(query_param("project_id", h.project.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schema_version": 1, "items": [unaccounted_binding], "next_cursor": null
        })))
        .mount(&h.backend.server())
        .await;
    // Re-use the concrete HTTP adapter held by a fresh guard by re-running the
    // existing guard: an unaccounted binding must prevent readiness reopening.
    let result = h
        .guard
        .reconcile(&ApprovedInventory { project: h.project }, &[h.project], &[])
        .await;
    assert_eq!(result, Err(RuntimeError::RuntimeAuthorityProtocolError));
    assert!(
        !h.guard.is_ready(),
        "G-8 a persistent prior ready state is not a restore proof"
    );
    h.backend.server().reset().await;
    let (status, body) = h.session(&client).await;
    assert_eq!(
        status, 502,
        "G-8 incomplete inventory cannot preserve legacy readiness: {body}"
    );
    assert!(body.get("session_token").is_none());
    assert_eq!(h.request_count().await, 0);
}
