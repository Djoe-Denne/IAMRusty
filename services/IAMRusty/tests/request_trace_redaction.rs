//! Request-level DEBUG capture around actual handlers and outbound error paths.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/owned_task.rs"]
mod owned_task;
mod utils;

use serde_json::{json, Value};
use serial_test::serial;
use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::instrument::WithSubscriber;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("capture lock")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

#[tokio::test]
#[serial]
async fn real_request_logs_do_not_expose_credentials_raw_oauth_urls_states_or_vendor_body() {
    let (fixture, _, _) = common::setup_test_server()
        .await
        .expect("real HTTP/Postgres fixture");
    fixture_cleanup::run(&fixture, async {
        let mut security = iam_configuration::security::SecurityConfig::default();
        security.mode = iam_configuration::security::SecurityMode::IsolatedTest;
        let app = common::build_test_iam_app(&fixture, security).await.expect("actual IAM app");
        let capture = Capture::default();
        let subscriber = Arc::new(tracing_subscriber::fmt().with_ansi(false).with_max_level(tracing::Level::DEBUG).with_writer(capture.clone()).finish());
        let request_subscriber = subscriber.clone();
        // Install on each real request future, not a global subscriber/reset.
        // Include a nonsecret witness so an empty capture cannot pass.
        let router = app.router().layer(axum::middleware::from_fn(move |request: axum::extract::Request, next: axum::middleware::Next| {
            let subscriber = request_subscriber.clone();
            async move { tracing::debug!(event="e4_request_capture", "request capture witness"); next.run(request).await }
                .with_subscriber(subscriber)
        }));
        let reserve = std::net::TcpListener::bind("127.0.0.1:0").expect("ephemeral test bind");
        let port = reserve.local_addr().expect("local address").port();
        drop(reserve);
        let config = rustycog::config::ServerConfig { host:"127.0.0.1".into(),port,tls_enabled:false,
            tls_cert_path:String::new(),tls_key_path:String::new(),tls_client_ca_path:String::new(),tls_require_client_cert:false,tls_port:0 };
        let handle = owned_task::spawn(rustycog::http::serve_router(router,config).with_subscriber(subscriber));
        let base = format!("http://127.0.0.1:{port}/iam");
        let client = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(3)).build().expect("test client");
        tokio::time::timeout(Duration::from_secs(10),async {
            loop {
                assert!(!handle.is_finished(),"owned test listener exited");
                if client.get(format!("{base}/health")).send().await.is_ok_and(|r|r.status().is_success()){break;}
                tokio::task::yield_now().await;
            }
        }).await.expect("bounded actual listener readiness");
        let login = client.post(format!("{base}/api/auth/login"))
            .json(&json!({"email":"SentinelABC-account@example.com","password":"SentinelABC-Password1a!"}))
            .send().await.expect("actual credential request");
        assert_eq!(login.status(),401);
        let idp = fixtures::IdpConnectFixtures::service().await;
        idp.mock_authorize("github").await;
        let vendor = rustycog::testing::wiremock::get_mock_server().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(format!("/github-connect{}",idp_connect_contract::dto::TOKEN_PATH)))
            .respond_with(wiremock::ResponseTemplate::new(500).set_body_string("SentinelABC-vendor-body-with-test-credential"))
            .mount(&vendor).await;
        let begin = client.get(format!("{base}/api/auth/github/login")).send().await.expect("actual BEGIN");
        assert_eq!(begin.status(),303);
        let cookie = begin.headers().get("set-cookie").expect("browser cookie").to_str().expect("cookie")
            .split(';').next().expect("cookie pair").to_owned();
        let location = begin.headers().get("location").expect("authorize URL").to_str().expect("URL").to_owned();
        let state = url::Url::parse(&location).expect("authorize URL").query_pairs().find(|(key,_)|key=="state").expect("actual persisted state").1.into_owned();
        let failed = client.get(format!("{base}/api/auth/github/callback"))
            .header("cookie",&cookie).query(&[("code","SentinelABC-authorization-code"),("state",state.as_str())])
            .send().await.unwrap_or_else(|_|panic!("callback transport failed (URL redacted)"));
        assert_eq!(failed.status(),401);
        let body: Value = failed.json().await.expect("generic error");
        assert_eq!(body["error"]["error_code"],"authentication_failed");
        assert_eq!(idp.received_requests().await.iter().filter(|r|r.url.path()==format!("/github-connect{}",idp_connect_contract::dto::TOKEN_PATH)).count(),1,
            "upstream sentinel body path must actually have been exercised");
        assert_eq!(client.get(format!("{base}/api/me")).bearer_auth("SentinelABC-invalid-bearer")
            .send().await.expect("Bearer rejection").status(),401);
        handle.abort();
        let _ = handle.await;
        let logs = capture.0.lock().expect("capture lock");
        let logs = String::from_utf8_lossy(&logs);
        assert!(logs.contains("e4_request_capture"),"request-level subscriber must capture a real event");
        for secret in ["SentinelABC",state.as_str(),cookie.as_str(),location.as_str()] {
            assert!(!logs.contains(secret),"DEBUG logs must redact request credentials, OAuth URLs/state and upstream bodies");
        }
    }).await;
}
