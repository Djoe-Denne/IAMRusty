//! M6 — une chaîne Kind : pin zot M1 → VALID JSON M2 → observateur M4 →
//! schedule M3 → invoke Lazaret M5 → T11 canary, **même digest catalogue 0002**.
//!
//! Pas six tests recopiés. Fail-loud si zot / JSON VALID / schedule / hop
//! invoke / sonde Kind T11 est sauté ou remplacé par un mock du hop.

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apparatus_operator::admission::{
    would_schedule, AdmissionStatus, AdmissionStore, PersistentAdmissionStore,
};
use apparatus_operator::admit::{push_and_sign_envelope, ENVELOPE_ARTIFACT_TYPE};
use apparatus_operator::controller::{
    cr_name_for, pod_name_for, IsolationLabels, ReconcileOutcome, WorkloadReconciler,
    BINDING_LABEL, PLUGINS_NAMESPACE, PLUGIN_INVOKE_PORT, PROJECT_LABEL, SYSTEM_NAMESPACE,
};
use apparatus_operator::desired_state::{
    reconcile_ready, DesiredStateSource, HttpComponentsClient, ReadyBinding,
};
use lazaret_application::{
    EnrollCommand, GrantService, IdentityService, InvokeService, PluginEndpointLocator,
};
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{ConnectorRegistry, WorkloadIdentity};
use lazaret_http::create_router;
use lazaret_infra::{
    DedicatedSessionSigner, DeniedSecretResolver, InMemoryEnrollmentRegistry, NamedConnectorProxy,
    PlatformInternalCa,
};
use serial_test::serial;
use uuid::Uuid;

use chain::{
    active_component_json, admit_target, app_state, assert_descriptor_shape, catalog_envelope,
    cri_hex, descriptor_from_disk, docker_build_reference_kv_http, invoke_body, passing_input,
    snapshot, spawn_manifesto_stub, wait_tcp, workload_csr, CountingLocator, MemoryPort, NoopKv,
    PortForward, RealCanary, TempStorePath, CRI_BUILD_TAG, CRI_IMAGE_NAME,
};

#[allow(dead_code)]
#[path = "fixtures/with_kind.rs"]
mod fixtures;

#[path = "fixtures/chain.rs"]
mod chain;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m6_e2e_0002_0008_chain() {
    let descriptor = descriptor_from_disk();
    assert_descriptor_shape(&descriptor);
    let catalog = descriptor.as_str();

    let cluster = fixtures::kind::KindCluster::ensure()
        .unwrap_or_else(|_| panic!("M6 FAIL cluster preconditions"));
    cluster
        .build_and_load_platform_plugin()
        .unwrap_or_else(|_| panic!("M6 FAIL platform image preparation"));
    docker_build_reference_kv_http();
    let cri_image = cluster
        .load_and_pin_image(CRI_BUILD_TAG, CRI_IMAGE_NAME)
        .unwrap_or_else(|_| panic!("M6 FAIL reference image preparation"));
    assert!(
        cri_image.contains("@sha256:"),
        "M1/M3 pin CRI @sha256 (pas latest): {cri_image}"
    );
    assert!(
        !cri_image.contains(":latest"),
        "cri_image sans latest: {cri_image}"
    );
    let cri = cri_hex(&cri_image);
    assert_ne!(
        cri,
        descriptor.hex_part(),
        "CRI @sha256 ≠ digest catalogue 0002 (cri={cri_image}, catalog={catalog})"
    );

    let stack = fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL H1 registry/Transit startup"));
    let signed = push_and_sign_envelope(
        &admit_target(&stack),
        &catalog_envelope(descriptor.clone(), cri_image.clone()),
    )
    .await
    .unwrap_or_else(|_| panic!("M6 FAIL H1 real push/sign/verify"));
    assert_eq!(
        signed.catalog_digest, descriptor,
        "identité catalogue = digest descripteur 0002"
    );
    assert!(
        signed.signature_verified,
        "cosign verify enveloppe (tag .sig ≠ preuve)"
    );
    assert_eq!(signed.artifact_type, ENVELOPE_ARTIFACT_TYPE);
    assert_ne!(
        signed.envelope_oci_digest.as_str(),
        catalog,
        "digest OCI enveloppe ≠ identité catalogue"
    );
    assert!(
        signed.envelope_oci_digest.starts_with("sha256:"),
        "enveloppe OCI={}",
        signed.envelope_oci_digest
    );
    let envelope_hex = signed
        .envelope_oci_digest
        .strip_prefix("sha256:")
        .expect("enveloppe sha256:");
    assert_ne!(
        envelope_hex, cri,
        "enveloppe OCI ≠ pin CRI (envelope={}, cri={cri_image})",
        signed.envelope_oci_digest
    );
    assert_eq!(signed.cri_image, cri_image);
    eprintln!(
        "{}",
        chain::evidence::h1(catalog, &signed.envelope_oci_digest, &signed.cri_image)
            .unwrap_or_else(|_| panic!("M6 FAIL H1 proof schema"))
    );

    let (mut input, from_disk) = passing_input();
    assert_eq!(from_disk, descriptor, "passing_input = même digest 0002");
    input.signature_verified = signed.signature_verified;

    let project_id = Uuid::new_v4();
    let component_id = Uuid::new_v4();
    let stub =
        spawn_manifesto_stub(active_component_json(project_id, component_id, &descriptor)).await;
    let source = HttpComponentsClient::new(stub.base_url, project_id, None::<String>)
        .unwrap_or_else(|_| panic!("M6 FAIL desired-state HTTP client"));
    let ready = source
        .list_ready()
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL desired-state HTTP list"));
    assert_eq!(
        ready,
        vec![ReadyBinding {
            project_id,
            component_id,
            digest: descriptor.clone(),
        }],
        "M4 ready = status==active + digest catalogue 0002"
    );

    let tmp = TempStorePath::new("valid-m1");
    let mut store = PersistentAdmissionStore::open(&tmp.path)
        .unwrap_or_else(|_| panic!("M6 FAIL admission store open"));
    let record = store
        .admit(input)
        .unwrap_or_else(|_| panic!("M6 FAIL admission validation"));
    assert_eq!(record.status, AdmissionStatus::Valid);
    assert_eq!(record.descriptor_digest, descriptor);
    store
        .bind_cri(&descriptor, &cri_image)
        .unwrap_or_else(|_| panic!("M6 FAIL bind CRI"));
    store
        .bind_envelope(&descriptor, &signed.reference)
        .unwrap_or_else(|_| panic!("M6 FAIL bind envelope"));
    assert!(
        would_schedule(&store, &descriptor),
        "would_schedule VALID pour le digest catalogue"
    );
    drop(store);
    let raw =
        fs::read_to_string(&tmp.path).unwrap_or_else(|_| panic!("M6 FAIL admission disk read"));
    assert!(
        raw.contains("VALID") && raw.contains(catalog),
        "M6 FAIL admission disk VALID/digest"
    );
    let store = PersistentAdmissionStore::open(&tmp.path)
        .unwrap_or_else(|_| panic!("M6 FAIL admission store reopen"));
    let got = store
        .get(&descriptor)
        .expect("GET produit VALID pour ce digest");
    assert_eq!(got.status, AdmissionStatus::Valid);
    assert_eq!(got.descriptor_digest, descriptor);
    assert!(would_schedule(&store, &descriptor));

    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &tmp.path)
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL reconciler connection"))
        .with_schedule_admit_target(admit_target(&stack));
    let pod_name = pod_name_for(&descriptor);
    let cr_name = cr_name_for(&descriptor);
    reconciler
        .delete_plugin_pod(&pod_name)
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL owned plugin pre-cleanup"));
    reconciler
        .delete_admission_record(&descriptor)
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL owned admission pre-cleanup"));

    let isolation = IsolationLabels::try_new(project_id.to_string(), component_id.to_string())
        .unwrap_or_else(|_| panic!("M6 FAIL isolation labels"));
    let outcomes = Box::pin(reconcile_ready(&source, &reconciler, &store))
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL desired-state scheduling"));
    assert_eq!(outcomes.len(), 1, "un binding ready");
    let schedule_status = match &outcomes[0] {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => {
            assert_eq!(*scheduled, pod_name);
            "Scheduled"
        }
        ReconcileOutcome::Refused(_) => {
            panic!("M6 FAIL schedule refused")
        }
    };
    let admission_status = if got.status == AdmissionStatus::Valid {
        "VALID"
    } else {
        "invalid"
    };
    eprintln!(
        "{}",
        chain::evidence::h2(
            got.descriptor_digest.as_str(),
            admission_status,
            schedule_status
        )
        .unwrap_or_else(|_| panic!("M6 FAIL H2 proof schema"))
    );
    reconciler
        .wait_pod_running(&pod_name, Duration::from_secs(180))
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL plugin Running wait"));

    let image = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.spec.containers[0].image}",
        ])
        .unwrap_or_else(|_| panic!("M6 FAIL plugin image read"));
    let image_txt = String::from_utf8_lossy(&image.stdout).trim().to_owned();
    chain::evidence::assert_api_equal(
        &image_txt,
        &cri_image,
        chain::evidence::ApiObservation::PluginImage,
    );

    let labels = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "--show-labels",
        ])
        .unwrap_or_else(|_| panic!("M6 FAIL plugin labels read"));
    let labels_txt = String::from_utf8_lossy(&labels.stdout);
    assert!(
        labels_txt.contains(&format!("{PROJECT_LABEL}={}", isolation.project)),
        "M6 FAIL project isolation label"
    );
    assert!(
        labels_txt.contains(&format!("{BINDING_LABEL}={}", isolation.binding)),
        "M6 FAIL binding isolation label"
    );

    let cr = cluster
        .kubectl(&[
            "get",
            "admissionrecord",
            &cr_name,
            "-n",
            SYSTEM_NAMESPACE,
            "-o",
            "jsonpath={.spec.descriptorDigest}",
        ])
        .unwrap_or_else(|_| panic!("M6 FAIL AdmissionRecord read"));
    let cr_digest = String::from_utf8_lossy(&cr.stdout).trim().to_owned();
    chain::evidence::assert_api_equal(
        &cr_digest,
        catalog,
        chain::evidence::ApiObservation::AdmissionDigest,
    );

    let local_port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind éphémère");
        listener.local_addr().expect("local_addr").port()
    };
    let pf = PortForward::new(
        cluster
            .spawn_kubectl(&[
                "port-forward",
                "--address=127.0.0.1",
                "-n",
                PLUGINS_NAMESPACE,
                &pod_name,
                &format!("{local_port}:{PLUGIN_INVOKE_PORT}"),
            ])
            .unwrap_or_else(|_| panic!("M6 FAIL plugin forward startup")),
    );
    wait_tcp(&format!("127.0.0.1:{local_port}"), Duration::from_secs(30));
    let plugin_url = format!("http://127.0.0.1:{local_port}");

    let hops = Arc::new(AtomicUsize::new(0));
    let locator: Arc<dyn PluginEndpointLocator> =
        Arc::new(CountingLocator::new(plugin_url, hops.clone()));

    let identity_body =
        WorkloadIdentity::try_new(Uuid::new_v4(), component_id, "release-1".to_owned(), 1, 1)
            .unwrap_or_else(|_| panic!("M6 FAIL workload identity"));
    let enroll_port = Arc::new(MemoryPort::new(snapshot(
        project_id,
        component_id,
        1,
        &["storage.kv.read", "storage.kv.write"],
        true,
    )));
    let cfg = IdentityConfig::default();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PlatformInternalCa::new().unwrap_or_else(|_| panic!("M6 FAIL internal CA"))),
        Arc::new(
            DedicatedSessionSigner::from_config(&cfg)
                .unwrap_or_else(|_| panic!("M6 FAIL session signer")),
        ),
        Arc::new(InMemoryEnrollmentRegistry::new()),
        enroll_port,
        cfg.session_ttl_minutes,
        cfg.cert_ttl_hours,
    ));
    let issued = identity
        .enroll(EnrollCommand {
            csr_pem: workload_csr(),
            identity: identity_body,
            project_id,
        })
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL enrollment"));
    let cert = lazaret_domain::VerifiedClientCertificate::from_pem(&issued.pem)
        .unwrap_or_else(|_| panic!("M6 FAIL certificate verification"));
    let token = identity
        .issue_session(&cert)
        .await
        .unwrap_or_else(|_| panic!("M6 FAIL session issuance"));

    let connectors = Arc::new(
        NamedConnectorProxy::new(ConnectorRegistry::default())
            .unwrap_or_else(|_| panic!("M6 FAIL connectors")),
    );
    let deny_grants = Arc::new(GrantService::new(Arc::new(MemoryPort::new(snapshot(
        project_id,
        component_id,
        1,
        &["storage.kv.read", "storage.kv.write"],
        false,
    )))));
    let allow_grants = Arc::new(GrantService::new(Arc::new(MemoryPort::new(snapshot(
        project_id,
        component_id,
        1,
        &["storage.kv.read", "storage.kv.write"],
        true,
    )))));
    let deny_invoke = Arc::new(InvokeService::new_with_locator(
        identity.clone(),
        deny_grants,
        Arc::new(NoopKv),
        Arc::new(DeniedSecretResolver),
        connectors.clone(),
        locator.clone(),
    ));
    let allow_invoke = Arc::new(InvokeService::new_with_locator(
        identity.clone(),
        allow_grants,
        Arc::new(NoopKv),
        Arc::new(DeniedSecretResolver),
        connectors,
        locator,
    ));
    let deny_server =
        axum_test::TestServer::new(create_router(app_state(), identity.clone(), deny_invoke))
            .unwrap_or_else(|_| panic!("M6 FAIL deny server startup"));
    let denied = deny_server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            component_id,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(denied.status_code(), 403, "M6 FAIL denied HTTP status");
    let denied_hops = hops.load(Ordering::SeqCst);
    assert_eq!(denied_hops, 0, "sans consentement : pas de hop INVOKE_PATH");

    let allow_server =
        axum_test::TestServer::new(create_router(app_state(), identity, allow_invoke))
            .unwrap_or_else(|_| panic!("M6 FAIL allow server startup"));
    let ok = allow_server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            component_id,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(ok.status_code(), 200, "M6 FAIL allowed HTTP status");
    assert!(
        hops.load(Ordering::SeqCst) >= 1,
        "grant allow doit localiser le plugin (INVOKE_PATH)"
    );
    let body: serde_json::Value =
        serde_json::from_str(&ok.text()).unwrap_or_else(|_| panic!("M6 FAIL invoke JSON"));
    let hostname = body["result"]["hostname"]
        .as_str()
        .unwrap_or_else(|| panic!("M6 FAIL hostname missing"));
    let h3 = chain::evidence::h3(
        denied.status_code().as_u16(),
        ok.status_code().as_u16(),
        denied_hops,
        hops.load(Ordering::SeqCst),
        hostname,
        &pod_name,
    )
    .unwrap_or_else(|_| panic!("M6 FAIL H3 HTTP/hops/hostname proof"));
    eprintln!("{h3}");
    let observation = chain::canary::observe_canary(&mut RealCanary {
        cluster: &cluster,
        plugin_pod: &pod_name,
    })
    .await
    .unwrap_or_else(|category| panic!("M6 FAIL H4 {category}"));
    eprintln!("{}", observation.marker());
    drop(pf);
    drop(stack);
}
