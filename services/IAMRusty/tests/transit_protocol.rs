//! S9 outbound HTTP protocol tests. No live OpenBao, ACL or HTTPS deployment claim.
use iam_domain::port::SigningProvider;
use iam_infra::signing::{StaticCredential, TransitSigningProvider};
use rustycog::testing::wiremock::MockServerFixture;
use serial_test::serial;
use std::{sync::Arc, time::Duration};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

struct TransitMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl TransitMockService {
    async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        Self {
            server: fixture.server(),
            _fixture: fixture,
        }
    }
    async fn sign_response(&self, key: &str, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path(format!("/v1/transit/sign/{key}")))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }
    fn provider(&self, key: &str, credential: &str) -> TransitSigningProvider {
        TransitSigningProvider::new(
            self.server.uri(),
            key,
            credential,
            Arc::new(
                StaticCredential::from_pair(
                    "org-fixture-credential",
                    "nonsecret-fixed-test-credential",
                )
                .expect("fixture credential"),
            ),
            None,
        )
        .expect("scoped client")
    }
}

#[tokio::test]
#[serial]
async fn missing_organization_credential_never_falls_back_or_calls_transit() {
    let fixture = TransitMockService::new().await;
    let key = format!("org-{}-signer", uuid::Uuid::new_v4());
    assert!(fixture
        .provider(&key, "missing-org-credential")
        .sign_digest(&[1; 32])
        .await
        .is_err());
    assert!(fixture
        .server
        .received_requests()
        .await
        .expect("request recording")
        .is_empty());
}

#[tokio::test]
#[serial]
async fn redirects_cannot_forward_credentials_to_an_alternate_operation() {
    let fixture = TransitMockService::new().await;
    let key = format!("org-{}-signer", uuid::Uuid::new_v4());
    let provider = fixture.provider(&key, "org-fixture-credential");
    fixture
        .sign_response(
            &key,
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data": {"signature": "vault:v1:AQID"}})),
        )
        .await;
    assert_eq!(
        provider
            .sign_digest(&[1; 32])
            .await
            .expect("positive Sign protocol control"),
        [1, 2, 3]
    );
    fixture._fixture.reset().await;
    fixture
        .sign_response(
            &key,
            ResponseTemplate::new(307).insert_header(
                "location",
                format!("{}/v1/transit/export/foreign", fixture.server.uri()),
            ),
        )
        .await;
    assert!(provider.sign_digest(&[1; 32]).await.is_err());
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .expect("requests")
            .len(),
        1
    );
}

#[tokio::test]
#[serial]
async fn transit_sign_has_bounded_timeout_without_alternate_retry() {
    let fixture = TransitMockService::new().await;
    let key = format!("org-{}-signer", uuid::Uuid::new_v4());
    fixture
        .sign_response(
            &key,
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30))
                .set_body_json(serde_json::json!({"data": {"signature": "vault:v1:AQID"}})),
        )
        .await;
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        fixture
            .provider(&key, "org-fixture-credential")
            .sign_digest(&[1; 32]),
    )
    .await
    .expect("client's 10-second timeout must precede the 15-second guard");
    assert!(result.is_err());
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .expect("requests")
            .len(),
        1
    );
}

#[tokio::test]
#[serial]
async fn unsafe_transit_paths_are_refused_before_any_http_request() {
    let fixture = TransitMockService::new().await;
    for key in [
        "../platform",
        "parent/foreign",
        "%2Fplatform",
        "apparatus-p4-cosign",
        "",
    ] {
        assert!(TransitSigningProvider::new(
            fixture.server.uri(),
            key,
            "org-fixture-credential",
            Arc::new(StaticCredential::default()),
            None
        )
        .is_err());
    }
    assert!(fixture
        .server
        .received_requests()
        .await
        .expect("requests")
        .is_empty());
}
