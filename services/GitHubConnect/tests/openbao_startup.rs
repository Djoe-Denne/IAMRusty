#![allow(missing_docs, clippy::future_not_send)]

mod common;

use axum_test::TestServer;
use common::fixtures::{
    github::GitHubFixtures,
    openbao::{resources::*, OpenBaoFixtures},
};
use common::{test_config, ALLOWED_REDIRECT};
use github_connect_configuration::{AppConfig, SecretReference, SecretsMode};
use github_connect_setup::Application;
use idp_connect_contract::{
    sign, unix_timestamp_secs, TokenRequest, SIGNATURE_HEADER, TIMESTAMP_HEADER,
};
use serial_test::serial;
use wiremock::ResponseTemplate;

const OAUTH: &str = "resolved-oauth-sentinel";
const HMAC: &str = "resolved-iam-hmac-sentinel";
const ROUTE: &str = "/github-connect/v1/token";

fn openbao_config(
    github: &str,
    bao: &common::fixtures::openbao::service::OpenBaoService,
) -> AppConfig {
    let mut config = test_config(github);
    config.github.secrets_mode = SecretsMode::Openbao;
    config.github.client_secret.clear();
    config.github.hmac_secret.clear();
    config.github.client_secret_ref = Some(
        SecretReference::new(
            "secret".into(),
            "github-connect/oauth".into(),
            "client_secret".into(),
        )
        .unwrap(),
    );
    config.github.hmac_secret_ref = Some(
        SecretReference::new(
            "secret".into(),
            "github-connect/iam".into(),
            "hmac_secret".into(),
        )
        .unwrap(),
    );
    config.openbao = Some(bao.config());
    config
}

async fn token_request(server: &TestServer, secret: &str) -> axum_test::TestResponse {
    let body = serde_json::to_string(&TokenRequest {
        code: "test_auth_code".into(),
        redirect_uri: ALLOWED_REDIRECT.into(),
        code_verifier: None,
    })
    .unwrap();
    let now = unix_timestamp_secs().unwrap();
    let signature = sign(secret.as_bytes(), "POST", ROUTE, now, &body).unwrap();
    server
        .post(ROUTE)
        .add_header(TIMESTAMP_HEADER, now.to_string())
        .add_header(SIGNATURE_HEADER, signature)
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(body))
        .await
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn resolved_secrets_reach_oauth_and_exact_iam_hmac_without_refresh() {
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    let bao = OpenBaoFixtures::service().await;
    bao.secret(OAUTH_PATH, "client_secret", OAUTH)
        .await
        .secret(HMAC_PATH, "hmac_secret", &format!("  {HMAC}\r\n"))
        .await;
    let app = Application::new(openbao_config(&github.base_url(), &bao))
        .await
        .expect("startup");
    assert!(app.config.github.client_secret.is_empty());
    assert!(app.config.github.hmac_secret.is_empty());
    let debug = format!("{:?}", app.config);
    let serialized = serde_json::to_string(&app.config).unwrap();
    for sentinel in [OAUTH, HMAC, BOOTSTRAP_TOKEN] {
        assert!(!debug.contains(sentinel));
        assert!(!serialized.contains(sentinel));
    }
    assert!(bao.request_count().await == 3);
    let server = TestServer::new(app.prefixed_router()).unwrap();
    token_request(&server, HMAC).await.assert_status_ok();
    token_request(&server, common::TEST_HMAC_SECRET)
        .await
        .assert_status_unauthorized();
    assert!(
        github
            .uses_oauth_credentials("test_github_client_id", OAUTH)
            .await
    );

    // Changing the vault does not refresh a running service. No business call reaches it.
    bao.reset().await;
    bao.secret(OAUTH_PATH, "client_secret", "replacement-oauth")
        .await
        .secret(HMAC_PATH, "hmac_secret", "replacement-hmac-secret")
        .await;
    token_request(&server, HMAC).await.assert_status_ok();
    token_request(&server, "replacement-hmac-secret")
        .await
        .assert_status_unauthorized();
    assert!(bao.request_count().await == 0);
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn acquisition_errors_never_return_an_application_or_leak_remote_content() {
    let github = GitHubFixtures::service().await;
    let bao = OpenBaoFixtures::service().await;
    for (endpoint, response) in [
        (
            OAUTH_PATH,
            ResponseTemplate::new(403).set_body_string(format!("{BOOTSTRAP_TOKEN} {OAUTH}")),
        ),
        (HMAC_PATH, ResponseTemplate::new(404)),
        (
            HMAC_PATH,
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data":{"data":{"hmac_secret":42}}})),
        ),
        (
            HMAC_PATH,
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data":{"data":{"hmac_secret":""}}})),
        ),
        (
            HMAC_PATH,
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data":{"data":{"hmac_secret":"too-short"}}})),
        ),
    ] {
        bao.reset().await;
        // Register the failure first (WireMock first-match semantics).
        bao.response(endpoint, response).await;
        bao.secret(OAUTH_PATH, "client_secret", OAUTH).await;
        let result = Application::new(openbao_config(&github.base_url(), &bao)).await;
        let error = result.err().expect("must refuse startup");
        let diagnostic = format!("{error:?} {error:#}");
        for sentinel in [BOOTSTRAP_TOKEN, OAUTH, HMAC] {
            assert!(!diagnostic.contains(sentinel));
        }
    }
    let mut config = openbao_config(&github.base_url(), &bao);
    config.openbao.as_mut().unwrap().endpoint = "http://127.0.0.1:0".into();
    assert!(Application::new(config).await.is_err());
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn explicit_direct_only_and_conflicts_refused_before_vault_io() {
    let bao = OpenBaoFixtures::service().await;
    let mut config = openbao_config("http://127.0.0.1:9", &bao);
    config.github.client_secret = "legacy-valid-client-secret".into();
    config.github.hmac_secret = common::TEST_HMAC_SECRET.into();
    assert!(Application::new(config.clone()).await.is_err());
    config.github.secrets_mode = SecretsMode::Direct;
    assert!(Application::new(config.clone()).await.is_err());
    config.github.secrets_mode = SecretsMode::Unconfigured;
    assert!(Application::new(config).await.is_err());
    assert!(bao.request_count().await == 0);
    let mut missing_reference = openbao_config("http://127.0.0.1:9", &bao);
    missing_reference.github.hmac_secret_ref = None;
    assert!(Application::new(missing_reference).await.is_err());
    assert!(bao.request_count().await == 0);
    let app = Application::new(test_config("http://127.0.0.1:9"))
        .await
        .expect("explicit direct");
    assert!(app.config.github.client_secret.is_empty());
    assert!(app.config.github.hmac_secret.is_empty());
}
