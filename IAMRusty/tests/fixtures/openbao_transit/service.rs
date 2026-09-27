//! Wiremock OpenBao Transit Sign mock.

use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::resources::*;

pub struct OpenBaoTransitMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl OpenBaoTransitMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
        }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    /// Stub `POST /v1/transit/sign/{name}` with a vault-style signature envelope.
    ///
    /// `signature_b64` is raw PKCS1v15 signature bytes, base64-encoded (standard).
    pub async fn mock_sign_ok(&self, key_name: &str, signature_b64: &str) -> &Self {
        let body = TransitSignResponse {
            data: TransitSignData {
                signature: format!("vault:v1:{signature_b64}"),
            },
        };
        Mock::given(method("POST"))
            .and(path(format!("/v1/transit/sign/{key_name}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub `POST /v1/transit/keys/{name}` create.
    pub async fn mock_create_key_ok(&self, key_name: &str) -> &Self {
        Mock::given(method("POST"))
            .and(path(format!("/v1/transit/keys/{key_name}")))
            .respond_with(ResponseTemplate::new(204))
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub `GET /v1/transit/keys/{name}` returning a public PEM.
    pub async fn mock_read_key_ok(&self, key_name: &str, public_key_pem: &str) -> &Self {
        Mock::given(method("GET"))
            .and(path(format!("/v1/transit/keys/{key_name}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "keys": {
                        "1": { "public_key": public_key_pem }
                    }
                }
            })))
            .mount(&*self.server)
            .await;
        self
    }
}
