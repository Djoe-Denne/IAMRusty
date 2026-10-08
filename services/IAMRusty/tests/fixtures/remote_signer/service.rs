//! `wiremock` remote `Sign` / `GetPublicKey` mocks (ADR-0309).

use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::resources::*;

pub struct RemoteSignerMockService {
    server: Arc<MockServer>,
    fixture: MockServerFixture,
}

impl RemoteSignerMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self { server, fixture }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn reset(&self) {
        self.fixture.reset().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    pub async fn mock_sign_ok(&self, signature_b64: &str) -> &Self {
        Mock::given(method("POST"))
            .and(path("/sign"))
            .respond_with(ResponseTemplate::new(200).set_body_json(SignResponseBody {
                signature: signature_b64.to_string(),
            }))
            .mount(&self.server)
            .await;
        self
    }

    pub async fn mock_get_public_key_ok(&self, public_key_pem: &str) -> &Self {
        self.mock_get_public_key_ok_for("kid-1", public_key_pem)
            .await
    }

    pub async fn mock_get_public_key_ok_for(&self, key_id: &str, public_key_pem: &str) -> &Self {
        Mock::given(method("GET"))
            .and(path(format!("/keys/{key_id}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(PublicKeyResponseBody {
                    public_key: public_key_pem.to_string(),
                }),
            )
            .mount(&self.server)
            .await;
        self
    }
}
