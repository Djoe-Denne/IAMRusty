//! Wiremock JWKS fixture (skill: `_fixture` + singleton port 3000).

use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

pub struct JwksFixtures {
    server: Arc<MockServer>,
    fixture: MockServerFixture,
}

impl JwksFixtures {
    pub async fn service() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self { server, fixture }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    pub async fn reset(&self) {
        self.fixture.reset().await;
    }

    pub async fn mock_jwks_ok(&self, body: &str) -> &Self {
        Mock::given(method("GET"))
            .and(path("/.well-known/jwks.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(body.to_string()),
            )
            .mount(&self.server)
            .await;
        self
    }
}
