//! C/G-8 real executable, real strict loader and owned Windows child.
//! Obligation -> risk -> scenario -> oracle:
//! - Real binary path -> test helper mistaken for service -> `CARGO_BIN_EXE` launch
//!   with isolated files/env -> invalid-source control and actual SDK preflight.
//! - Run admits only complete boot -> fake port0/reserved-then-closed port -> parse
//!   effective IPv4 address from bounded logs after bind -> health/ready and signed v2.
//! - Owned process -> deadlock/leaked executable -> separate bounded drains and
//!   early exit/timeout/panic -> child-only kill/reap/drain before test failure.
//!
//! KEEP. Windows signal shutdown is NOT asserted; Application SDK join is separate.

#[path = "support/d_process.rs"]
mod d_process;
#[path = "../setup/tests/support/d_vendor.rs"]
mod d_vendor;

use d_process::Process;
use d_vendor::{body, clean, must, signature, source, Vendors, DISCOVERY, KEY_OAUTH, KEY_OIDC};
use idp_connect_contract::hmac::{SIGNATURE_HEADER, TIMESTAMP_HEADER};
use oauth_connect_configuration::load_config;
use serde_json::Value;
use std::{net::SocketAddr, path::Path, time::Duration};

const LOADER_TEST: &str = "d8_binary_input_loader_preflight_isolated_child";
const LOADER_MARKER: &str = "OC_SERVICE_LOADER_PREFLIGHT";

fn directory() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("isolated child cwd");
    std::fs::create_dir(dir.path().join("config")).expect("isolated config directory");
    dir
}
fn write_source(cwd: &Path, vendors: &Vendors) {
    std::fs::write(cwd.join("config/test.toml"), source(&vendors.config()))
        .expect("isolated synthetic service input");
    std::fs::write(
        cwd.join("config/development.toml"),
        "wrong-source='sentinel'\n",
    )
    .expect("wrong RUN_ENV trap");
}
fn loader_preflight(cwd: &Path) {
    let mut child = Process::spawn(
        &std::env::current_exe().expect("integration test executable"),
        cwd,
        &["--exact", LOADER_TEST, "--nocapture"],
        Some((LOADER_MARKER, LOADER_TEST)),
    );
    let result = child.finish(Duration::from_secs(8));
    assert!(
        result.success,
        "actual SDK source preflight failed; NOT an Application RED"
    );
    assert!(
        result.contains("running 1 test") && result.contains("SERVICE_INPUT_VALIDATED"),
        "must execute the real loader oracle, not merely launch a test executable"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d8_binary_input_loader_preflight_isolated_child() {
    if std::env::var(LOADER_MARKER).is_ok_and(|v| v == LOADER_TEST) {
        let loaded = must(load_config(), "real SDK service input loader");
        assert_eq!(loaded.providers.len(), 2);
        assert_eq!(loaded.providers[0].id, "forge");
        assert_eq!(loaded.providers[1].id, "authority");
        assert_eq!(loaded.server.port, 0);
        assert_eq!(loaded.server.host, "127.0.0.1");
        assert_eq!(loaded.logging.level, "info");
        println!("SERVICE_INPUT_VALIDATED");
        return;
    }
    let vendors = Vendors::new().await;
    let dir = directory();
    write_source(dir.path(), &vendors);
    loader_preflight(dir.path());
    assert!(vendors.requests(true).await.is_empty());
    assert!(vendors.requests(false).await.is_empty());
}

fn address(text: &str) -> Option<SocketAddr> {
    text.lines()
        .filter(|line| {
            let words = line.to_ascii_lowercase();
            words.contains("listen") || words.contains("bound") || words.contains("serving")
        })
        .flat_map(|line| line.split(|c: char| !c.is_ascii_digit() && c != '.' && c != ':'))
        .filter_map(|word| word.parse::<SocketAddr>().ok())
        .find(|a| a.is_ipv4() && a.ip().is_loopback() && a.port() != 0)
}

async fn bytes(mut response: reqwest::Response) -> (reqwest::StatusCode, Vec<u8>) {
    let status = response.status();
    let mut result = Vec::new();
    while let Some(chunk) = must(response.chunk().await, "bounded binary HTTP response") {
        assert!(
            chunk.len() <= 8192 - result.len(),
            "binary response too large"
        );
        result.extend_from_slice(&chunk);
    }
    (status, result)
}
async fn exercise(origin: &str, vendors: &Vendors) {
    let http = must(
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(4))
            .build(),
        "incoming binary test HTTP client",
    );
    for route in ["health", "ready"] {
        let (status, response) = bytes(must(
            http.get(format!("{origin}/oauth-connect/{route}"))
                .send()
                .await,
            "binary platform request",
        ))
        .await;
        assert_eq!(status, reqwest::StatusCode::OK);
        clean(&String::from_utf8_lossy(&response));
        if route == "ready" {
            let report: Value = must(serde_json::from_slice(&response), "binary readiness JSON");
            assert!(report["status"].as_str() == Some("ready"));
        }
    }
    for (id, key, oidc) in [("forge", KEY_OAUTH, false), ("authority", KEY_OIDC, true)] {
        for completion in [false, true] {
            let operation = if completion { "complete" } else { "authorize" };
            let path = format!("/oauth-connect/{id}/v2/{operation}");
            let body = body(completion);
            let (at, mac) = signature(&path, &body, key);
            let response = must(
                http.post(format!("{origin}{path}"))
                    .header("content-type", "application/json")
                    .header(TIMESTAMP_HEADER, at.to_string())
                    .header(SIGNATURE_HEADER, mac)
                    .body(body)
                    .send()
                    .await,
                "binary signed v2",
            );
            let (status, response) = bytes(response).await;
            assert_eq!(status, reqwest::StatusCode::OK);
            let value: Value = must(serde_json::from_slice(&response), "binary response DTO");
            if completion {
                vendors.assert_profile(&value, oidc);
            } else {
                let url = must(
                    url::Url::parse(
                        value["authorization_url"]
                            .as_str()
                            .expect("binary authorize URL"),
                    ),
                    "binary authorize parse",
                );
                assert_eq!(url.path(), "/authorize");
                let expected_origin = if oidc {
                    vendors.oidc.uri()
                } else {
                    vendors.oauth.uri()
                };
                assert_eq!(
                    url.origin(),
                    must(url::Url::parse(&expected_origin), "fixture origin").origin()
                );
            }
        }
        vendors.credentials(oidc).await;
        assert_eq!(vendors.count(oidc, "/token").await, 1);
    }
    assert_eq!(vendors.count(true, DISCOVERY).await, 1);
    assert_eq!(vendors.count(true, "/keys").await, 1);
    let before = vendors.count(true, "/token").await;
    let path = "/oauth-connect/authority/v2/complete";
    let body = body(true);
    let (at, mac) = signature(path, &body, KEY_OAUTH);
    let response = must(
        http.post(format!("{origin}{path}"))
            .header("content-type", "application/json")
            .header(TIMESTAMP_HEADER, at.to_string())
            .header(SIGNATURE_HEADER, mac)
            .body(body)
            .send()
            .await,
        "binary wrong-instance-key request",
    );
    let (status, response) = bytes(response).await;
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
    assert!(
        response.is_empty(),
        "wrong HMAC must return no contextual content"
    );
    assert_eq!(
        vendors.count(true, "/token").await,
        before,
        "wrong HMAC must not reach vendor token endpoint"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d8_real_binary_late_boot_failure_exits_without_listen_address() {
    let vendors = Vendors::new().await;
    let dir = directory();
    write_source(dir.path(), &vendors);
    loader_preflight(dir.path());
    vendors.oidc.reset().await;
    let mut discovery = d_vendor::metadata(&vendors.oidc.uri());
    discovery["issuer"] = Value::String("http://127.0.0.1:1/wrong".into());
    d_vendor::mount(
        &vendors.oidc,
        "GET",
        DISCOVERY,
        wiremock::ResponseTemplate::new(200).set_body_json(discovery),
    )
    .await;
    let binary = Path::new(env!("CARGO_BIN_EXE_oauth-connect-service"));
    assert!(
        binary.is_file(),
        "real service executable missing: NOT product RED"
    );
    let mut process = Process::spawn(binary, dir.path(), &[], None);
    let mut emitted_address = false;
    let _ = process.observe_until(Duration::from_secs(8), |text| {
        emitted_address |= address(text).is_some();
        false
    });
    let result = process.finish(Duration::from_secs(1));
    assert!(
        !result.success,
        "late boot failure must not exit successfully"
    );
    assert!(
        !result.contains("Invalid OAuthConnect configuration"),
        "validated input must reach late boot, not configuration rejection"
    );
    assert!(
        vendors.count(true, DISCOVERY).await >= 1,
        "late boot failure must actually reach Discovery; T0 refusal alone is insufficient"
    );
    assert!(
        !emitted_address,
        "late boot failure must not admit or log a listener"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d8_real_binary_port_zero_logs_bound_address_then_ready_and_signed_oauth_oidc() {
    let vendors = Vendors::new().await;
    let dir = directory();
    write_source(dir.path(), &vendors);
    loader_preflight(dir.path());
    let binary = Path::new(env!("CARGO_BIN_EXE_oauth-connect-service"));
    assert!(
        binary.is_file(),
        "real service executable missing: NOT product RED"
    );
    let mut process = Process::spawn(binary, dir.path(), &[], None);
    let mut bound = None;
    let observed = process.observe_until(Duration::from_secs(12), |text| {
        bound = address(text);
        bound.is_some()
    });
    if !observed {
        let result = process.finish(Duration::from_secs(1));
        assert!(
            !result.contains("Invalid OAuthConnect configuration"),
            "valid source was rejected before Application"
        );
        assert!(result.contains("OAuthConnect service startup not implemented"),
            "startup did not reach the known T0 Application refusal; investigate, not a qualified RED");
        panic!("validated service input reached T0 but no effective bound address was logged");
    }
    let origin = format!("http://{}", bound.expect("observed actual address"));
    // Assertions run in an owned task so even a panic first closes/reaps this child.
    let mut assertions = tokio::spawn(async move {
        exercise(&origin, &vendors).await;
    });
    let result = tokio::time::timeout(Duration::from_secs(20), &mut assertions).await;
    if result.is_err() {
        assertions.abort();
        let _ = assertions.await;
    }
    // No proven portable Windows graceful-signal path: child-only kill/reap is permitted fallback.
    let _ = process.finish(Duration::ZERO);
    result
        .unwrap_or_else(|_| panic!("binary assertions timed out"))
        .unwrap_or_else(|e| {
            if e.is_panic() {
                std::panic::resume_unwind(e.into_panic());
            }
            panic!("binary assertion task cancelled");
        });
}

#[test]
fn d8_real_binary_invalid_source_exits_before_any_listen_address() {
    let dir = directory();
    std::fs::write(dir.path().join("config/test.toml"), "providers=[]\n")
        .expect("invalid service input");
    let binary = Path::new(env!("CARGO_BIN_EXE_oauth-connect-service"));
    let mut process = Process::spawn(binary, dir.path(), &[], None);
    let mut emitted_address = false;
    let _ = process.observe_until(Duration::from_secs(5), |text| {
        emitted_address |= address(text).is_some();
        false
    });
    let result = process.finish(Duration::from_secs(1));
    assert!(
        !result.success,
        "invalid source binary unexpectedly succeeded"
    );
    assert!(
        result.contains("Invalid OAuthConnect configuration"),
        "must be the real strict configuration failure"
    );
    assert!(
        !emitted_address,
        "invalid source must not admit or log a listener"
    );
}
