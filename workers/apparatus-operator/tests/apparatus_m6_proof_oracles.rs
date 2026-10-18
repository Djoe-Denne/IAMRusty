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

struct Script {
    trace: Vec<&'static str>,
    ready: bool,
    endpoints: Result<Vec<u8>, &'static str>,
    positive: Result<PositiveHttp, &'static str>,
    negative: Result<WgetOutput, &'static str>,
    endpoint_reads: usize,
    changed_endpoint: Option<(usize, Vec<u8>)>,
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
    async fn positive_http(&mut self, target: &CanaryTarget) -> Result<PositiveHttp, &'static str> {
        self.trace.push("positive");
        assert_eq!(
            target.url(),
            "http://10.244.0.42:8080/",
            "positive and negative identify the READY endpoint"
        );
        assert_eq!(target.pod, "apparatus-system-canary-6c98b8d5f7-fixture");
        std::mem::replace(&mut self.positive, Err("positive consumed"))
    }
    async fn wget(&mut self, target: &CanaryTarget) -> Result<WgetOutput, &'static str> {
        self.trace.push("wget");
        assert_eq!(target.url(), "http://10.244.0.42:8080/");
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
            "positive",
            "endpoints",
            "wget",
            "endpoints"
        ]
    );
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
                    &["ready", "endpoints", "positive"],
                    "canary positive HTTP failed",
                )
            }
            "wrong-status" => {
                port.positive = Ok(PositiveHttp {
                    status: 503,
                    body: "404 Not Found".to_owned(),
                });
                (
                    &["ready", "endpoints", "positive"],
                    "canary positive HTTP unexpected response",
                )
            }
            _ => {
                port.positive = Ok(PositiveHttp {
                    status: 404,
                    body: "Bearer secret-token".to_owned(),
                });
                (
                    &["ready", "endpoints", "positive"],
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
            ["ready", "endpoints", "positive", "endpoints", "wget"]
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
