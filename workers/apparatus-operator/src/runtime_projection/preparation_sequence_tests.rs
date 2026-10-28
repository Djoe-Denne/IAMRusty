//! Owner-inline sequencing tests compiled as a child of the production module.
//! Fake HTTP/control I/O is NOT a TLS, SQL, Adm-A or real Serving proof. The
//! private attempt entry isolates already-qualified inputs from real admission.
use super::*;
use crate::admission::{AdmissionRecord, AdmissionStatus};
use apparatus_contracts::runtime::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Delayed,
    NotReady,
    Ready,
    SlowCreate,
    SlowCreateNoMetadata,
    NeverMetadata,
    CreateTimeout,
    Conflict,
    NoUid,
    LeaseLost,
    CasLost,
    ForeignParent,
    ForeignOwner,
    LostCommitted,
    LostUncommitted,
    PutTimeout,
    SlowMetadata,
}
struct Trace {
    case: Case,
    start: Instant,
    creates: Vec<Instant>,
    reads: Vec<Instant>,
    renews: Vec<Instant>,
    reports: Vec<RuntimeObservationV1>,
    gets_after_report: usize,
    mutations: Vec<String>,
    snapshot: RuntimeBindingSnapshotV1,
    lease: RuntimeWriterLeaseV1,
}
#[derive(Clone)]
struct Control(Arc<Mutex<Trace>>);
fn fail(code: RuntimeErrorCodeV1) -> RuntimeControlErrorV1 {
    code.error(
        RuntimeServiceRoleV1::Projector,
        RuntimeEffectOutcomeV1::NotStarted,
    )
}
macro_rules! unused_port {
    ($name:ident($($arg:ident:$ty:ty),*)->$result:ty)=>{
        fn $name<'a>(&'a self,$($arg:$ty),*)->RuntimeControlFuture<'a,$result>{
            $(let _=$arg;)* Box::pin(async{Err(fail(RuntimeErrorCodeV1::RuntimeServiceForbidden))})
        }
    };
}
impl RuntimeControlPort for Control {
    fn get_binding<'a>(
        &'a self,
        _caller: &'a RuntimeControlCallerV1,
        _project: Uuid,
        _binding: Uuid,
        _instances: &'a RuntimeInstanceSelectorV1,
    ) -> RuntimeControlFuture<'a, RuntimeBindingSnapshotV1> {
        Box::pin(async move {
            let mut trace = self.0.lock().expect("trace");
            if !trace.reports.is_empty() {
                trace.gets_after_report += 1;
            }
            let mut snapshot = trace.snapshot.clone();
            if trace.case == Case::CasLost
                && Instant::now().duration_since(trace.start) >= Duration::from_secs(3)
            {
                snapshot.cas.revocation_epoch =
                    snapshot.cas.revocation_epoch.checked_next().expect("epoch");
            }
            Ok(snapshot)
        })
    }
    fn writer_lease<'a>(
        &'a self,
        _caller: &'a RuntimeControlCallerV1,
        _project: Uuid,
        _binding: Uuid,
        request: &'a RuntimeWriterLeaseRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriterLeaseV1> {
        Box::pin(async move {
            let mut trace = self.0.lock().expect("trace");
            assert_eq!(request.holder_id, trace.lease.proof.holder_id);
            if request.renew.is_some() {
                trace.renews.push(Instant::now());
                if trace.case == Case::LeaseLost {
                    return Err(fail(RuntimeErrorCodeV1::RuntimeLeaseLost));
                }
                let issued =
                    1 + i64::try_from(Instant::now().duration_since(trace.start).as_millis())
                        .expect("milliseconds");
                trace.lease.issued_at_unix_ms = RuntimeUnixMsV1::new(issued).expect("issued");
                trace.lease.expires_at_unix_ms = RuntimeUnixMsV1::new(issued + 10000).expect("ttl");
            }
            Ok(trace.lease.clone())
        })
    }
    fn report_observation<'a>(
        &'a self,
        _caller: &'a RuntimeControlCallerV1,
        _project: Uuid,
        _binding: Uuid,
        request: &'a RuntimeObservationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            let (case, ack) = {
                let mut trace = self.0.lock().expect("trace");
                trace.reports.push(request.clone());
                assert_eq!(request.service_uid, "service-uid");
                assert_eq!(request.revision_uid, "revision-uid");
                let mut cas = trace.snapshot.cas.clone();
                cas.projection_version = cas.projection_version.checked_next().expect("version");
                let ack = RuntimeWriteAckV1 {
                    schema_version: 1,
                    project_id: request.project_id,
                    binding_id: request.binding_id,
                    cas: cas.clone(),
                    writer_fence: request.guard.lease.writer_fence,
                };
                if !matches!(trace.case, Case::LostUncommitted | Case::PutTimeout) {
                    trace.snapshot.cas = cas.clone();
                    trace.snapshot.observation = Some(RuntimeObservationSnapshotV1 {
                        schema_version: 1,
                        project_id: request.project_id,
                        binding_id: request.binding_id,
                        cas,
                        writer_fence: request.guard.lease.writer_fence,
                        service_uid: request.service_uid.clone(),
                        resource_generation: request.resource_generation,
                        observed_resource_generation: request.observed_resource_generation,
                        revision_uid: request.revision_uid.clone(),
                        revision_name: request.revision_name.clone(),
                        release: request.release.clone(),
                        endpoint: request.endpoint.clone(),
                        route_ready: request.route_ready,
                        observed_at_unix_ms: request.observed_at_unix_ms,
                        valid_until_unix_ms: RuntimeUnixMsV1::new(
                            request.observed_at_unix_ms.get() + 5000,
                        )
                        .expect("until"),
                    });
                }
                (trace.case, ack)
            };
            if case == Case::PutTimeout {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            if matches!(case, Case::LostCommitted | Case::LostUncommitted) {
                Err(fail(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable))
            } else {
                Ok(ack)
            }
        })
    }
    unused_port!(list_bindings(caller:&'a RuntimeControlCallerV1,project:Uuid,cursor:Option<&'a RuntimeCursorV1>,size:Option<u16>)->RuntimeBindingPageV1);
    unused_port!(register_instance(caller:&'a RuntimeControlCallerV1,project:Uuid,binding:Uuid,instance:Uuid,request:&'a RuntimeInstanceRegistrationV1)->RuntimeWriteAckV1);
    unused_port!(revoke_instance(caller:&'a RuntimeControlCallerV1,project:Uuid,binding:Uuid,instance:Uuid,request:&'a RuntimeInstanceRevokeRequestV1)->RuntimeWriteAckV1);
    unused_port!(begin_effect(caller:&'a RuntimeControlCallerV1,project:Uuid,binding:Uuid,request:&'a RuntimeBeginEffectRequestV1)->RuntimeEffectReservationV1);
    unused_port!(complete_effect(caller:&'a RuntimeControlCallerV1,project:Uuid,binding:Uuid,effect:Uuid,request:&'a RuntimeCompleteEffectRequestV1)->RuntimeWriteAckV1);
}

struct Admission(AdmissionRecord);
impl CurrentServingAdmissionSource for Admission {
    fn load_current(
        &self,
        digest: &crate::digest::ReleaseDigest,
    ) -> Result<AdmissionRecord, RuntimeErrorCodeV1> {
        if digest != &self.0.descriptor_digest {
            return Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted);
        }
        Ok(self.0.clone())
    }
}

struct Harness {
    bridge: ServingRuntimeBridge,
    permit: RuntimeBindingPreparationPermitV1,
    snapshot: RuntimeBindingSnapshotV1,
    image: AdmittedServingImage,
    lease: RuntimeWriterLeaseV1,
    trace: Arc<Mutex<Trace>>,
}
impl Harness {
    fn new(case: Case) -> Self {
        let count = RuntimeCounterV1::new(1).expect("count");
        let release = RuntimeDigestV1::new(format!("sha256:{}", "1".repeat(64))).expect("release");
        let snapshot = RuntimeBindingSnapshotV1 {
            schema_version: 1,
            project_id: Uuid::new_v4(),
            binding_id: Uuid::new_v4(),
            cas: RuntimeCasV1 {
                generation: count,
                grant_revision: count,
                revocation_epoch: count,
                projection_version: count,
            },
            state: RuntimeBindingStateV1::Closed,
            release: release.clone(),
            profile: "test-profile".to_owned(),
            observation: None,
            instances: None,
        };
        let lock = RuntimeDigestV1::new(format!("sha256:{}", "2".repeat(64))).expect("lock");
        let permit = RuntimeBindingPreparationPermitV1 {
            migration_id: Uuid::new_v4(),
            project_id: snapshot.project_id,
            binding_id: snapshot.binding_id,
            cas: snapshot.cas.clone(),
            release: release.clone(),
            profile: snapshot.profile.clone(),
            profile_lock_sha256: lock.clone(),
            inventory_sha256: lock.clone(),
        };
        let record = AdmissionRecord {
            descriptor_digest: crate::digest::ReleaseDigest::new(release.as_str()).expect("digest"),
            policy_version: POLICY_ID.to_owned(),
            report_digest: crate::digest::ReleaseDigest::new(lock.as_str()).expect("report"),
            status: AdmissionStatus::Valid,
            cri_image: Some(format!("registry.test/ref@{}", release.as_str())),
            envelope_reference: Some(format!("registry.test/envelope@{}", lock.as_str())),
        };
        let image = AdmittedServingImage {
            image: record.cri_image.clone().expect("image"),
            descriptor: release.as_str().to_owned(),
            record: Some(record.clone()),
        };
        let profile = ServingPlatformProfile::new(
            "test-profile".to_owned(),
            "https://gateway.test/lazaret".to_owned(),
            "runtime-ca".to_owned(),
        )
        .expect("profile")
        .with_approved_preparation(
            lock,
            ServingEndpointPolicy::new(
                "apparatus-plugins.svc.cluster.local".to_owned(),
                443,
                ServingEndpointTransport::Https,
            )
            .expect("endpoint"),
        );
        let lease = RuntimeWriterLeaseV1 {
            schema_version: 1,
            project_id: snapshot.project_id,
            binding_id: snapshot.binding_id,
            proof: RuntimeLeaseProofV1 {
                lease_id: Uuid::new_v4(),
                holder_id: Uuid::new_v4(),
                writer_fence: count,
            },
            cas: snapshot.cas.clone(),
            issued_at_unix_ms: RuntimeUnixMsV1::new(1).expect("at"),
            expires_at_unix_ms: RuntimeUnixMsV1::new(10001).expect("ttl"),
        };
        let mut parent =
            render_parent(&snapshot, &image, &profile, count, Some(&permit)).expect("parent");
        parent["metadata"]["uid"] = json!("service-uid");
        parent["metadata"]["resourceVersion"] = json!("opaque-rv");
        parent["metadata"]["generation"] = json!(1);
        let revision_name = parent["spec"]["template"]["metadata"]["name"]
            .as_str()
            .expect("revision")
            .to_owned();
        let name = serving_name(&snapshot);
        parent["status"] = json!({"observedGeneration":1,"latestCreatedRevisionName":revision_name,"latestReadyRevisionName":revision_name,
            "conditions":[{"type":"Ready","status":if case==Case::NotReady{"False"}else{"True"}}],"address":{"url":format!("https://{name}.apparatus-plugins.svc.cluster.local/")}});
        let revision = json!({"apiVersion":"serving.knative.dev/v1","kind":"Revision","metadata":{"name":revision_name,"namespace":"apparatus-plugins","uid":"revision-uid",
            "ownerReferences":[{"apiVersion":"serving.knative.dev/v1","kind":"Configuration","name":"config","uid":"config-uid","controller":true}]},
            "spec":{"containers":[{"name":"user-container","image":image.image}]}});
        let config = json!({"apiVersion":"serving.knative.dev/v1","kind":"Configuration","metadata":{"name":"config","namespace":"apparatus-plugins","uid":"config-uid",
            "ownerReferences":[{"apiVersion":"serving.knative.dev/v1","kind":"Service","name":name,"uid":if case==Case::ForeignOwner{"foreign-uid"}else{"service-uid"},"controller":true}]}});
        let trace = Arc::new(Mutex::new(Trace {
            case,
            start: Instant::now(),
            creates: vec![],
            reads: vec![],
            renews: vec![],
            reports: vec![],
            gets_after_report: 0,
            mutations: vec![],
            snapshot: snapshot.clone(),
            lease: lease.clone(),
        }));
        let service = tower::service_fn({
            let trace = trace.clone();
            move |request: axum::http::Request<kube::client::Body>| {
                let trace = trace.clone();
                let parent = parent.clone();
                let revision = revision.clone();
                let config = config.clone();
                async move {
                    let method = request.method().to_string();
                    let path = request.uri().path().to_owned();
                    let (case, elapsed) = {
                        let mut t = trace.lock().expect("trace");
                        let now = Instant::now();
                        if method == "POST" {
                            t.creates.push(now);
                        } else if path.contains("/revisions/") {
                            t.reads.push(now);
                        }
                        if method != "GET" {
                            t.mutations.push(method.clone());
                        }
                        (t.case, now.duration_since(t.start))
                    };
                    let mut status = 200;
                    let mut value: Value = parent;
                    if method == "POST" {
                        if case == Case::CreateTimeout {
                            tokio::time::sleep(Duration::from_secs(31)).await;
                        }
                        if matches!(case, Case::SlowCreate | Case::SlowCreateNoMetadata) {
                            tokio::time::sleep(Duration::from_millis(2500)).await;
                        }
                        if case == Case::Conflict {
                            status = 409;
                        }
                        if case == Case::NoUid {
                            value["metadata"]
                                .as_object_mut()
                                .expect("metadata")
                                .remove("uid");
                        }
                    } else if path.contains("/revisions/") {
                        if case == Case::SlowMetadata {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                        }
                        if matches!(
                            case,
                            Case::NeverMetadata
                                | Case::LeaseLost
                                | Case::CasLost
                                | Case::SlowMetadata
                                | Case::SlowCreateNoMetadata
                        ) || case == Case::Delayed && elapsed < Duration::from_secs(5)
                        {
                            status = 404;
                        } else {
                            value = revision;
                        }
                    } else if path.contains("/configurations/") {
                        value = config;
                    } else if case == Case::ForeignParent {
                        value["metadata"]["uid"] = json!("foreign-parent");
                    }
                    if method == "GET" && path.contains("/services/") && case == Case::Ready {
                        value["status"]["conditions"][0]["status"] = json!("True");
                    }
                    if status != 200 {
                        value = json!({"apiVersion":"v1","kind":"Status","status":"Failure","reason":if status==404{"NotFound"}else{"AlreadyExists"},"message":"test","code":status});
                    }
                    Ok::<_, std::io::Error>(
                        axum::http::Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(axum::body::Body::from(value.to_string()))
                            .expect("response"),
                    )
                }
            }
        });
        let client = kube::Client::new(service, "apparatus-plugins");
        // TEST-only target: costly Cosign admission is deliberately outside this
        // private sequencing oracle. No claim of real admission/TLS is made.
        let target = crate::admit::AdmitTarget {
            registry_host: "registry.test".to_owned(),
            registry_network: "registry.test".to_owned(),
            docker_network: String::new(),
            vault_addr_host: String::new(),
            vault_addr_network: String::new(),
            vault_token: String::new(),
            transit_key: String::new(),
            repository: "envelope".to_owned(),
            tag: String::new(),
            auth: crate::admit::RegistryAuth {
                username: String::new(),
                password: String::new(),
            },
        };
        let mut bridge = ServingRuntimeBridge::new(
            Arc::new(Control(trace.clone())),
            ServingParentApi::new(client),
            Arc::new(Admission(record)),
            Arc::new(target),
            profile,
        );
        bridge.holder_id = lease.proof.holder_id;
        Self {
            bridge,
            permit,
            snapshot,
            image,
            lease,
            trace,
        }
    }
    async fn run(&self) -> Result<RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        let start = self.trace.lock().expect("trace").start;
        self.bridge
            .prepare_attempt(
                &self.permit,
                &self.snapshot,
                &self.image,
                self.lease.clone(),
                start,
            )
            .await
    }
    fn invariant(&self) {
        let t = self.trace.lock().expect("trace");
        assert_eq!(t.creates.len(), 1);
        assert!(t.mutations.iter().all(|method| method == "POST"));
        assert!(t.reads.len() <= 28);
        assert!(t
            .reads
            .windows(2)
            .all(|pair| pair[1].duration_since(pair[0]) >= Duration::from_secs(1)));
        assert!(t
            .reads
            .iter()
            .all(|at| at.duration_since(t.start) < Duration::from_secs(28)));
        assert!(Instant::now().duration_since(t.start) <= Duration::from_secs(30));
        assert!(t.gets_after_report <= 1);
    }
}

#[tokio::test(start_paused = true)]
async fn runtime_preparation_delayed_revision_and_coordinated_lease_renewal() {
    let h = Harness::new(Case::Delayed);
    assert!(h.run().await.is_ok());
    h.invariant();
    let t = h.trace.lock().expect("trace");
    assert_eq!(t.reports.len(), 1);
    assert!(t
        .renews
        .iter()
        .any(|at| at.duration_since(t.start) == Duration::from_secs(3)));
    assert!(t.reads.len() > 1);
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_complete_not_ready_reports_without_open() {
    let h = Harness::new(Case::NotReady);
    assert!(h.run().await.is_ok());
    h.invariant();
    assert!(!h.trace.lock().expect("trace").reports[0].route_ready);
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_ready_report_is_not_an_open_command() {
    let h = Harness::new(Case::Ready);
    assert!(h.run().await.is_ok());
    h.invariant();
    assert_eq!(
        h.trace.lock().expect("trace").snapshot.state,
        RuntimeBindingStateV1::Closed
    );
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_create_2500_ms_does_not_reset_clock() {
    let h = Harness::new(Case::SlowCreate);
    assert!(h.run().await.is_ok());
    h.invariant();
    assert_eq!(
        Instant::now().duration_since(h.trace.lock().expect("trace").start),
        Duration::from_millis(2500)
    );
    let h = Harness::new(Case::SlowCreateNoMetadata);
    assert!(h.run().await.is_err());
    h.invariant();
    assert_eq!(
        Instant::now().duration_since(h.trace.lock().expect("trace").start),
        Duration::from_secs(28),
        "discovery deadline cannot reset after the 2500ms create response"
    );
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_discovery_stops_at_28s_without_partial_report() {
    let h = Harness::new(Case::NeverMetadata);
    assert!(h.run().await.is_err());
    h.invariant();
    let t = h.trace.lock().expect("trace");
    assert_eq!(
        Instant::now().duration_since(t.start),
        Duration::from_secs(28)
    );
    assert!(t.reports.is_empty());
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_create_deadline_30s_includes_create_and_no_retry() {
    let h = Harness::new(Case::CreateTimeout);
    assert!(h.run().await.is_err());
    h.invariant();
    assert_eq!(
        Instant::now().duration_since(h.trace.lock().expect("trace").start),
        Duration::from_secs(30)
    );
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_lost_lease_and_changed_cas_stop_attempt() {
    for case in [Case::LeaseLost, Case::CasLost] {
        let h = Harness::new(case);
        assert!(h.run().await.is_err());
        h.invariant();
        assert!(h.trace.lock().expect("trace").reports.is_empty());
    }
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_create_conflict_or_missing_uid_never_adopts() {
    for case in [Case::Conflict, Case::NoUid] {
        let h = Harness::new(case);
        assert!(h.run().await.is_err());
        h.invariant();
        assert!(h.trace.lock().expect("trace").reports.is_empty());
    }
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_foreign_same_name_parent_is_not_deleted() {
    let h = Harness::new(Case::ForeignParent);
    assert!(h.run().await.is_err());
    h.invariant();
    assert!(h.trace.lock().expect("trace").reports.is_empty());
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_foreign_owner_chain_never_reports_or_mutates_child() {
    let h = Harness::new(Case::ForeignOwner);
    assert!(h.run().await.is_err());
    h.invariant();
    assert!(h.trace.lock().expect("trace").reports.is_empty());
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_committed_put_lost_ack_has_one_confirmation() {
    let h = Harness::new(Case::LostCommitted);
    assert!(h.run().await.is_ok());
    h.invariant();
    assert_eq!(h.trace.lock().expect("trace").gets_after_report, 1);
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_uncommitted_put_lost_ack_stays_unknown() {
    let h = Harness::new(Case::LostUncommitted);
    assert!(h.run().await.is_err());
    h.invariant();
    let t = h.trace.lock().expect("trace");
    assert_eq!(t.gets_after_report, 1);
    assert!(t.snapshot.observation.is_none());
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_cancel_before_report_keeps_unknown_residual() {
    let h = Harness::new(Case::NeverMetadata);
    assert!(tokio::time::timeout(Duration::from_secs(5), h.run())
        .await
        .is_err());
    h.invariant();
    let t = h.trace.lock().expect("trace");
    assert!(t.reports.is_empty());
    assert!(t.snapshot.observation.is_none());
    assert!(require_absent_for_create(&t.snapshot, true).is_err());
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_committed_report_keeps_durable_uid_not_local_receipt() {
    let h = Harness::new(Case::LostCommitted);
    assert!(h.run().await.is_ok());
    h.invariant();
    let t = h.trace.lock().expect("trace");
    assert_eq!(
        t.snapshot
            .observation
            .as_ref()
            .expect("durable")
            .service_uid,
        "service-uid"
    );
    assert!(matches!(
        require_absent_for_create(&t.snapshot, true),
        Err(RuntimeErrorCodeV1::RuntimeStaleState)
    ));
}
#[tokio::test(start_paused = true)]
async fn runtime_preparation_put_timeout_never_extends_confirmation_budget() {
    let h = Harness::new(Case::PutTimeout);
    assert!(h.run().await.is_err());
    h.invariant();
    let t = h.trace.lock().expect("trace");
    assert_eq!(t.gets_after_report, 0);
    assert_eq!(
        Instant::now().duration_since(t.start),
        Duration::from_secs(2)
    );
}

#[tokio::test(start_paused = true)]
async fn runtime_preparation_old_capture_is_not_reused_when_fresh_metadata_missing() {
    let h = Harness::new(Case::LostCommitted);
    assert!(h.run().await.is_ok());
    let snapshot = {
        let mut trace = h.trace.lock().expect("trace");
        trace.case = Case::NeverMetadata;
        trace.snapshot.clone()
    };
    let old = snapshot.observation.as_ref().expect("old capture");
    let observer = RuntimePodObserver::new(h.bridge.parents.parents.clone().into_client());
    let guard = RuntimeWriteGuardV1 {
        expected: snapshot.cas.clone(),
        lease: h.lease.proof.clone(),
    };
    let mut chain = PreparationChainUids::default();
    let fresh = observer
        .preparation_metadata(
            &snapshot,
            &old.service_uid,
            &old.revision_name,
            &h.image.image,
            &guard,
            &h.bridge.profile,
            &mut chain,
        )
        .await
        .expect("incomplete read");
    assert!(
        fresh.is_none(),
        "a stored observation cannot become a fresh heartbeat"
    );
    assert_eq!(h.trace.lock().expect("trace").reports.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn runtime_preparation_each_read_round_is_cancelled_at_2s() {
    let h = Harness::new(Case::SlowMetadata);
    assert!(h.run().await.is_err());
    h.invariant();
    let trace = h.trace.lock().expect("trace");
    assert!(trace.reports.is_empty());
    assert!(trace.reads.len() > 1);
    assert!(trace
        .reads
        .windows(2)
        .all(|pair| pair[1].duration_since(pair[0]) <= Duration::from_secs(2)));
}

#[tokio::test(start_paused = true)]
async fn runtime_preparation_later_ready_is_new_capture_and_still_not_open() {
    let h = Harness::new(Case::NotReady);
    assert!(h.run().await.is_ok());
    let snapshot = {
        let mut trace = h.trace.lock().expect("trace");
        trace.case = Case::Ready;
        trace.snapshot.clone()
    };
    let old = snapshot
        .observation
        .as_ref()
        .expect("durable false capture");
    assert!(!old.route_ready);
    tokio::time::advance(Duration::from_secs(2)).await;
    let observer = RuntimePodObserver::new(h.bridge.parents.parents.clone().into_client());
    let guard = RuntimeWriteGuardV1 {
        expected: snapshot.cas.clone(),
        lease: h.lease.proof.clone(),
    };
    let mut chain = PreparationChainUids::default();
    let fresh = observer
        .preparation_metadata(
            &snapshot,
            &old.service_uid,
            &old.revision_name,
            &h.image.image,
            &guard,
            &h.bridge.profile,
            &mut chain,
        )
        .await
        .expect("read")
        .expect("complete");
    assert!(fresh.route_ready);
    assert_eq!(
        h.trace.lock().expect("trace").snapshot.state,
        RuntimeBindingStateV1::Closed
    );
    assert_eq!(
        h.trace.lock().expect("trace").reports.len(),
        1,
        "capture alone is not an Open/report command"
    );
}
