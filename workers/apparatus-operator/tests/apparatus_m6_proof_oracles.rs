//! PH-6/7: exercise the same causal orchestration and closed proof schema as m6.
//! These scripted observations do not replace the real ORAS/services/Pod chain.

#[allow(dead_code)]
#[path = "fixtures/canary.rs"]
mod canary;
#[path = "fixtures/evidence.rs"]
mod evidence;

use async_trait::async_trait;
use canary::{observe_canary, CanaryPort, CanaryTarget, PositiveHttp, WgetOutput};

const ENDPOINTS: &[u8] = br#"{"subsets":[{"addresses":[{"ip":"10.244.0.42","targetRef":{"kind":"Pod","namespace":"apparatus-system","uid":"12345678-1234-4234-8234-123456789abc","name":"apparatus-system-canary-6c98b8d5f7-fixture"}}],"ports":[{"port":8080,"protocol":"TCP"}]}]}"#;
const POD: &[u8] = br#"{"kind":"Pod","metadata":{"name":"apparatus-system-canary-6c98b8d5f7-fixture","namespace":"apparatus-system","uid":"12345678-1234-4234-8234-123456789abc"},"status":{"podIP":"10.244.0.42","conditions":[{"type":"Ready","status":"True"}]}}"#;

struct Script {
    trace: Vec<&'static str>,
    ready: bool,
    endpoints: Result<Vec<u8>, &'static str>,
    positive: Result<PositiveHttp, &'static str>,
    negative: Result<WgetOutput, &'static str>,
    endpoint_reads: usize,
    changed_endpoint: Option<(usize, Vec<u8>)>,
    pod: Result<Vec<u8>, &'static str>,
    pod_reads: usize,
    changed_pod: Option<(usize, Result<Vec<u8>, &'static str>)>,
    wget_urls: Vec<String>,
}

impl Script {
    fn nominal() -> Self {
        Self {
            trace: Vec::new(),
            ready: true,
            endpoints: Ok(ENDPOINTS.to_vec()),
            positive: Ok(PositiveHttp {
                status: 404,
                body: "<HTML><H1>404 Not Found</H1></HTML>".to_owned(),
            }),
            negative: Ok(WgetOutput {
                code: Some(4),
                stdout: "M6_EXEC_READY\nM6_WGET_GNU\n\nM6_WGET_EXIT=4\n".to_owned(),
                stderr: "Connecting to 10.244.0.42:8080... failed: Connection timed out.\n"
                    .to_owned(),
            }),
            endpoint_reads: 0,
            changed_endpoint: None,
            pod: Ok(POD.to_vec()),
            pod_reads: 0,
            changed_pod: None,
            wget_urls: Vec::new(),
        }
    }
}

#[async_trait]
impl CanaryPort for Script {
    async fn wait_ready(&mut self) -> Result<(), &'static str> {
        self.trace.push("ready");
        if self.ready {
            Ok(())
        } else {
            Err("private raw diagnostic")
        }
    }
    async fn endpoints(&mut self) -> Result<Vec<u8>, &'static str> {
        self.trace.push("endpoints");
        self.endpoint_reads += 1;
        if let Some((at, value)) = &self.changed_endpoint {
            if self.endpoint_reads == *at {
                return Ok(value.clone());
            }
        }
        self.endpoints.clone()
    }
    async fn pod(&mut self, name: &str) -> Result<Vec<u8>, &'static str> {
        self.trace.push("pod");
        assert_eq!(
            name, "apparatus-system-canary-6c98b8d5f7-fixture",
            "S-1 actual Pod API requested by referenced name"
        );
        self.pod_reads += 1;
        if let Some((at, value)) = &self.changed_pod {
            if self.pod_reads == *at {
                return value.clone();
            }
        }
        self.pod.clone()
    }
    async fn positive_http(&mut self, target: &CanaryTarget) -> Result<PositiveHttp, &'static str> {
        self.trace.push("positive");
        // Like the real port-forward, this positive addresses the POD NAME,
        // not Endpoints.ip. S-1's deceptive A-positive/B-timeout stays possible
        // in this script unless the shared orchestration checks Pod API identity.
        assert_eq!(target.pod, "apparatus-system-canary-6c98b8d5f7-fixture");
        std::mem::replace(&mut self.positive, Err("positive consumed"))
    }
    async fn wget(&mut self, target: &CanaryTarget) -> Result<WgetOutput, &'static str> {
        self.trace.push("wget");
        self.wget_urls.push(target.url());
        std::mem::replace(&mut self.negative, Err("wget consumed"))
    }
}

#[tokio::test]
async fn ph7_nominal_proves_preconditions_before_bounded_wget_observation() {
    // Mutation: wget-before-ready, ignored positive response or generic nonzero
    // success changes the recorded order or the rejected cases in the next tests.
    let mut port = Script::nominal();
    let proof = observe_canary(&mut port)
        .await
        .expect("identified remote wget connect timeout");
    assert_eq!(
        port.trace,
        [
            "ready",
            "endpoints",
            "pod",
            "positive",
            "endpoints",
            "pod",
            "wget",
            "endpoints",
            "pod"
        ]
    );
    assert_eq!(port.wget_urls, ["http://10.244.0.42:8080/"]);
    assert_eq!(
        proof.marker(),
        "M6 H4 positive_http=404 negative=wget-connect-timeout-bounded"
    );
}

#[tokio::test]
async fn ph7_failed_preconditions_block_wget_and_success_marker() {
    for scenario in [
        "readiness",
        "endpoint-read",
        "empty",
        "not-ready",
        "wrong-port",
        "malformed",
        "positive",
        "wrong-status",
        "wrong-body",
    ] {
        let mut port = Script::nominal();
        let (expected_trace, category): (&[&str], &str) = match scenario {
            "readiness" => {
                port.ready = false;
                (&["ready"], "canary readiness failed")
            }
            "endpoint-read" => {
                port.endpoints = Err("private diagnostic");
                (&["ready", "endpoints"], "canary endpoints read failed")
            }
            "empty" => {
                port.endpoints = Ok(br#"{"subsets":[]}"#.to_vec());
                (&["ready", "endpoints"], "canary ready endpoints absent")
            }
            "not-ready" => {
                port.endpoints = Ok(br#"{"subsets":[{"notReadyAddresses":[{"ip":"10.244.0.42"}],"ports":[{"port":8080,"protocol":"TCP"}]}]}"#.to_vec());
                (&["ready", "endpoints"], "canary ready endpoints absent")
            }
            "wrong-port" => {
                port.endpoints = Ok(String::from_utf8_lossy(ENDPOINTS)
                    .replace("8080", "9090")
                    .into_bytes());
                (&["ready", "endpoints"], "canary ready endpoints absent")
            }
            "malformed" => {
                port.endpoints = Ok(b"private malformed payload".to_vec());
                (&["ready", "endpoints"], "canary endpoints invalid")
            }
            "positive" => {
                port.positive = Err("private diagnostic");
                (
                    &["ready", "endpoints", "pod", "positive"],
                    "canary positive HTTP failed",
                )
            }
            "wrong-status" => {
                port.positive = Ok(PositiveHttp {
                    status: 503,
                    body: "404 Not Found".to_owned(),
                });
                (
                    &["ready", "endpoints", "pod", "positive"],
                    "canary positive HTTP unexpected response",
                )
            }
            _ => {
                port.positive = Ok(PositiveHttp {
                    status: 404,
                    body: "Bearer secret-token".to_owned(),
                });
                (
                    &["ready", "endpoints", "pod", "positive"],
                    "canary positive HTTP unexpected response",
                )
            }
        };
        let published = observe_canary(&mut port).await.map(|proof| proof.marker());
        assert_eq!(
            published.expect_err("PH-7 no success marker with missing precondition"),
            category,
            "{scenario}"
        );
        assert_eq!(
            port.trace, expected_trace,
            "PH-7 {scenario} stops before wget"
        );
    }
}

#[tokio::test]
async fn ph7_reached_unavailable_dns_tool_exec_routing_and_transport_errors_never_prove_block() {
    let diagnostics = [
        "HTTP/1.1 200 OK",
        "HTTP/1.1 404 Not Found",
        "Connection refused",
        "can't connect to remote host",
        "network is unreachable",
        "No route to host",
        "wget: bad address",
        "unable to resolve host address",
        "wget: not found",
        "executable file not found",
        "error executing command",
        "unable to upgrade connection",
        "i/o timeout",
        "context deadline exceeded",
        "Error from server (Timeout): request did not complete",
        "timed out waiting for the condition",
        "wget: download timed out",
        "wget: timeout",
        "unclassified failure",
    ];
    for diagnostic in diagnostics {
        let mut port = Script::nominal();
        // Even a valid footer/code cannot turn an unrelated failure into a DROP.
        port.negative.as_mut().expect("scenario output").stderr = diagnostic.to_owned();
        let result = observe_canary(&mut port).await.map(|proof| proof.marker());
        assert!(
            result.is_err(),
            "PH-7 non-probative diagnostic must fail, never emit H4: {diagnostic}"
        );
        assert_eq!(
            port.trace,
            [
                "ready",
                "endpoints",
                "pod",
                "positive",
                "endpoints",
                "pod",
                "wget"
            ]
        );
    }
}

#[tokio::test]
async fn ph7_timeout_requires_completed_exec_exit4_and_exact_ready_target() {
    for scenario in [
        "transport-error",
        "no-start",
        "no-footer",
        "wrong-tool",
        "duplicate-start",
        "duplicate-footer",
        "no-exit",
        "wrong-exit",
        "wrong-target",
        "contradiction",
        "success-exit",
        "disappeared-before",
        "disappeared-after",
        "replaced-before",
        "replaced-after",
    ] {
        let mut port = Script::nominal();
        if scenario.starts_with("disappeared-") || scenario.starts_with("replaced-") {
            let at = if scenario.ends_with("before") { 2 } else { 3 };
            let endpoints = if scenario.starts_with("disappeared-") {
                br#"{"subsets":[]}"#.to_vec()
            } else {
                String::from_utf8_lossy(ENDPOINTS)
                    .replace("123456789abc", "123456789def")
                    .into_bytes()
            };
            port.changed_endpoint = Some((at, endpoints));
        } else if scenario == "transport-error" {
            port.negative = Err("private raw transport diagnostic");
        } else {
            let output = port.negative.as_mut().expect("output");
            match scenario {
                "no-start" => output.stdout = "M6_WGET_EXIT=4\n".to_owned(),
                "no-footer" => output.stdout = "M6_EXEC_READY\nM6_WGET_GNU\n".to_owned(),
                "wrong-tool" => {
                    output.stdout = output.stdout.replace("M6_WGET_GNU", "M6_WGET_BUSYBOX");
                }
                "duplicate-start" => output.stdout.insert_str(0, "M6_EXEC_READY\n"),
                "duplicate-footer" => output.stdout.push_str("M6_WGET_EXIT=4\n"),
                "no-exit" => output.code = None,
                "wrong-exit" => output.code = Some(127),
                "success-exit" => output.code = Some(0),
                "wrong-target" => {
                    output.stderr =
                        "Connecting to 10.244.0.99:8080... failed: Connection timed out."
                            .to_owned();
                }
                _ => output.stderr.push_str("Connection refused"),
            }
        }
        assert!(
            observe_canary(&mut port)
                .await
                .map(|proof| proof.marker())
                .is_err(),
            "PH-7 incomplete/ambiguous timeout {scenario}"
        );
        if scenario.ends_with("before") {
            assert!(
                !port.trace.contains(&"wget"),
                "endpoint changed before wget prevents exec"
            );
        }
    }
}

#[test]
fn ph6_closed_h1_h2_h3_markers_use_values_and_reject_secret_shaped_input() {
    let catalog = format!("sha256:{}", "a".repeat(64));
    let envelope = format!("sha256:{}", "b".repeat(64));
    let image = format!("apparatus-reference-kv@sha256:{}", "c".repeat(64));
    let hostname = format!("plugin-{}", "a".repeat(32));
    assert_eq!(
        evidence::h1(&catalog, &envelope, &image).expect("H1"),
        format!("M6 H1 catalog={catalog} envelope={envelope} image={image}")
    );
    assert_eq!(
        evidence::h2(&catalog, "VALID", "Scheduled").expect("H2"),
        format!("M6 H2 catalog={catalog} status=VALID schedule=Scheduled")
    );
    assert_eq!(evidence::h3(403, 200, 0, 2, &hostname, &hostname).expect("H3"), format!("M6 H3 denied_http=403 allowed_http=200 denied_hops=0 allowed_hops=2 hostname={hostname}"));
    for secret in [
        "Bearer secret-token",
        "sha256:deadbeef\r\nAUTH=secret",
        "password=secret",
        "-----BEGIN PRIVATE KEY-----",
    ] {
        let error = evidence::h1(secret, &envelope, &image).expect_err("no raw secret in H1");
        assert_eq!(error, "H1 digest or image invalid");
        assert!(evidence::h1(&catalog, &envelope, secret).is_err());
        assert_eq!(
            evidence::h2(&catalog, secret, "Scheduled").expect_err("closed status"),
            "H2 observation invalid"
        );
        assert_eq!(
            evidence::h3(403, 200, 0, 1, secret, secret).expect_err("closed hostname"),
            "H3 observation invalid"
        );
    }
    assert!(evidence::h1(&catalog, &envelope, "apparatus-reference-kv:latest").is_err());
    assert!(evidence::h2(&catalog, "REFUSED", "Scheduled").is_err());
    assert!(evidence::h2(&catalog, "VALID", "Refused").is_err());
    for (denied, allowed, deny_hops, allow_hops, observed) in [
        (200, 200, 0, 1, hostname.as_str()),
        (403, 500, 0, 1, hostname.as_str()),
        (403, 200, 1, 1, hostname.as_str()),
        (403, 200, 0, 0, hostname.as_str()),
        (403, 200, 0, 1, "plugin-unexpected"),
    ] {
        assert!(
            evidence::h3(denied, allowed, deny_hops, allow_hops, observed, &hostname).is_err(),
            "H3 must encode the observed contract, not a success constant"
        );
    }
}

fn pod_fault(scenario: &str) -> (Result<Vec<u8>, &'static str>, &'static str) {
    let mut pod: serde_json::Value =
        serde_json::from_slice(POD).expect("S-1 independent Pod fixture");
    let category = match scenario {
        "ip" => {
            pod["status"]["podIP"] = "10.244.0.99".into();
            "canary Pod IP mismatch"
        }
        "uid" => {
            pod["metadata"]["uid"] = "12345678-1234-4234-8234-123456789def".into();
            "canary Pod identity mismatch"
        }
        "name" => {
            pod["metadata"]["name"] = "foreign-pod-abc".into();
            "canary Pod identity mismatch"
        }
        "namespace" => {
            pod["metadata"]["namespace"] = "foreign-namespace".into();
            "canary Pod identity mismatch"
        }
        "kind" => {
            pod["kind"] = "Service".into();
            "canary Pod identity mismatch"
        }
        "not-ready" => {
            pod["status"]["conditions"][0]["status"] = "False".into();
            "canary Pod not Ready"
        }
        "no-ready-condition" => {
            pod["status"]["conditions"] = serde_json::json!([]);
            "canary Pod not Ready"
        }
        "terminating" => {
            pod["metadata"]["deletionTimestamp"] = "2026-10-18T16:00:00Z".into();
            "canary Pod terminating"
        }
        "unavailable" => {
            return (
                Err("synthetic API read failure"),
                "canary Pod API read failed",
            )
        }
        "malformed" => {
            return (
                Ok(b"synthetic malformed Pod JSON".to_vec()),
                "canary Pod API invalid",
            )
        }
        _ => panic!("unknown S-1 fixture scenario"),
    };
    (
        Ok(serde_json::to_vec(&pod).expect("Pod fixture JSON")),
        category,
    )
}

#[tokio::test]
async fn s1_pod_identity_readiness_and_termination_are_verified_before_positive() {
    // Mutation: trusting Endpoints targetRef or Ready alone would allow these
    // stable Endpoints replies to proceed to a positive for the wrong/unready Pod.
    for scenario in [
        "ip",
        "uid",
        "name",
        "namespace",
        "kind",
        "not-ready",
        "no-ready-condition",
        "terminating",
        "unavailable",
        "malformed",
    ] {
        let mut port = Script::nominal();
        let (reply, category) = pod_fault(scenario);
        port.pod = reply;
        let result = observe_canary(&mut port).await.map(|proof| proof.marker());
        assert_eq!(
            result.expect_err("S-1 no H4 with invalid Pod API"),
            category,
            "{scenario}"
        );
        assert_eq!(
            port.trace,
            ["ready", "endpoints", "pod"],
            "S-1 no positive forward or wget: {scenario}"
        );
        assert!(port.wget_urls.is_empty());
    }
}

#[tokio::test]
async fn s1_stable_forged_endpoint_ip_cannot_combine_positive_pod_a_and_timeout_ip_b() {
    let mut port = Script::nominal();
    // Attacker controls ONLY Endpoints: name/namespace/UID still refer to living
    // Pod A. Positive A is scripted 404 and negative B a valid wget timeout, so
    // without the independent Pod API comparison the old false H4 is possible.
    port.endpoints = Ok(String::from_utf8_lossy(ENDPOINTS)
        .replace("10.244.0.42", "10.244.0.99")
        .into_bytes());
    port.negative.as_mut().expect("scripted timeout B").stderr =
        "Connecting to 10.244.0.99:8080... failed: Connection timed out.\n".to_owned();
    let result = observe_canary(&mut port).await.map(|proof| proof.marker());
    assert_eq!(
        result.expect_err("S-1 forged B must not yield H4"),
        "canary Pod IP mismatch"
    );
    assert_eq!(
        port.trace,
        ["ready", "endpoints", "pod"],
        "known mismatch prevents the positive forward as well as wget"
    );
    assert_eq!(
        port.pod,
        Ok(POD.to_vec()),
        "Pod A remains unchanged independently of Endpoints B"
    );
}

#[tokio::test]
async fn s1_pod_replacement_or_loss_is_rechecked_with_unchanged_endpoints() {
    for read in [2, 3] {
        for scenario in ["uid", "ip", "not-ready", "terminating", "unavailable"] {
            let mut port = Script::nominal();
            let (reply, category) = pod_fault(scenario);
            port.changed_pod = Some((read, reply));
            let result = observe_canary(&mut port).await.map(|proof| proof.marker());
            assert_eq!(
                result.expect_err("S-1 revalidation prevents H4"),
                category,
                "Pod read {read}: {scenario}"
            );
            assert_eq!(
                port.endpoints,
                Ok(ENDPOINTS.to_vec()),
                "Endpoints unchanged, independent Pod API changed"
            );
            assert_eq!(port.pod_reads, read, "stop immediately at changed Pod read");
            let expected: &[&str] = if read == 2 {
                &["ready", "endpoints", "pod", "positive", "endpoints", "pod"]
            } else {
                &[
                    "ready",
                    "endpoints",
                    "pod",
                    "positive",
                    "endpoints",
                    "pod",
                    "wget",
                    "endpoints",
                    "pod",
                ]
            };
            assert_eq!(port.trace, expected, "S-1 causal stop at {scenario}");
            assert_eq!(
                port.wget_urls.len(),
                usize::from(read == 3),
                "before-negative discrepancy prevents wget"
            );
        }
    }
}

#[tokio::test]
async fn s1_ready_pod_secondary_ip_is_supported_and_revalidated() {
    let mut port = Script::nominal();
    let mut pod: serde_json::Value = serde_json::from_slice(POD).expect("Pod fixture");
    pod["status"]["podIP"] = "fd00::42".into();
    pod["status"]["podIPs"] = serde_json::json!([{"ip":"fd00::42"},{"ip":"10.244.0.42"}]);
    port.pod = Ok(serde_json::to_vec(&pod).expect("secondary IP fixture"));
    let proof = observe_canary(&mut port)
        .await
        .expect("numeric target belongs to this Ready Pod's podIPs");
    assert_eq!(
        proof.marker(),
        "M6 H4 positive_http=404 negative=wget-connect-timeout-bounded"
    );
    assert_eq!(
        port.pod_reads, 3,
        "Pod checked before positive and before/after wget"
    );
    assert_eq!(port.wget_urls, ["http://10.244.0.42:8080/"]);
}

#[tokio::test]
async fn t3_foreign_pod_or_namespace_is_rejected_before_pod_api_and_positive_forward() {
    for (from, to) in [
        (
            "apparatus-system-canary-6c98b8d5f7-fixture",
            "foreign-pod-abc",
        ),
        (
            "\"namespace\":\"apparatus-system\"",
            "\"namespace\":\"foreign-namespace\"",
        ),
    ] {
        let mut port = Script::nominal();
        port.endpoints = Ok(String::from_utf8_lossy(ENDPOINTS)
            .replace(from, to)
            .into_bytes());
        assert_eq!(
            observe_canary(&mut port)
                .await
                .expect_err("T-3 foreign targetRef"),
            "canary ready endpoints absent"
        );
        assert_eq!(
            port.trace,
            ["ready", "endpoints"],
            "T-3 early rejection before Pod API/forward/wget"
        );
        assert!(port.wget_urls.is_empty());
    }
}

#[test]
fn s2_api_equality_panics_keep_failure_category_without_either_operand() {
    for (observation, category, sentinel) in [
        (
            evidence::ApiObservation::PluginImage,
            "M6 FAIL plugin image mismatch",
            "SYNTHETIC_IMAGE_OPERAND_SENTINEL",
        ),
        (
            evidence::ApiObservation::AdmissionDigest,
            "M6 FAIL AdmissionRecord digest mismatch",
            "SYNTHETIC_CR_OPERAND_SENTINEL",
        ),
    ] {
        evidence::assert_api_equal("same-observed-value", "same-observed-value", observation);
        let expected = "SYNTHETIC_EXPECTED_OPERAND_SENTINEL";
        let panic = std::panic::catch_unwind(|| {
            evidence::assert_api_equal(sentinel, expected, observation);
        })
        .expect_err("S-2 unequal API operands MUST still fail");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("S-2 string panic category");
        assert_eq!(message, category, "S-2 closed FAIL category retained");
        assert!(
            !message.contains(sentinel),
            "S-2 received value must not be reflected"
        );
        assert!(
            !message.contains(expected),
            "S-2 expected value must not be reflected"
        );
    }
}
