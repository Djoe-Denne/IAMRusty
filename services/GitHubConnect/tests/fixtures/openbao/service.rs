use super::resources::*;
use github_connect_configuration::{OpenBaoAuthConfig, OpenBaoConfig};
use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

pub struct OpenBaoService {
    server: Arc<MockServer>,
    fixture: MockServerFixture,
    token_file: tempfile::NamedTempFile,
}

impl OpenBaoService {
    /// Create an isolated HTTP fixture with a synthetic credential file.
    ///
    /// # Panics
    /// Panics if the temporary test credential cannot be created or written.
    pub async fn new() -> Self {
        let fixture = MockServerFixture::isolated().await;
        let token_file = tempfile::NamedTempFile::new().expect("synthetic credential file");
        std::fs::write(token_file.path(), BOOTSTRAP_TOKEN).expect("synthetic credential");
        let service = Self {
            server: fixture.server(),
            fixture,
            token_file,
        };
        service.lookup().await;
        service
    }
    pub fn config(&self) -> OpenBaoConfig {
        OpenBaoConfig {
            endpoint: self.server.uri(),
            auth: OpenBaoAuthConfig::TokenFile {
                path: self.token_file.path().to_owned(),
            },
            allow_loopback_http: true,
            ca_cert_file: None,
        }
    }
    async fn lookup(&self) {
        Mock::given(method("GET"))
            .and(path("/v1/auth/token/lookup-self"))
            .and(header("X-Vault-Token", BOOTSTRAP_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(Lookup {
                data: LookupData {
                    policies: vec!["github-connect-read"],
                    ttl: 600,
                },
            }))
            .mount(&self.server)
            .await;
    }
    pub async fn secret(&self, endpoint: &str, field: &str, value: &str) -> &Self {
        self.response(
            endpoint,
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data":{"data":{field:value}}})),
        )
        .await
    }
    pub async fn response(&self, endpoint: &str, response: ResponseTemplate) -> &Self {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .and(header("X-Vault-Token", BOOTSTRAP_TOKEN))
            .respond_with(response)
            .mount(&self.server)
            .await;
        self
    }
    pub async fn reset(&self) {
        self.fixture.reset().await;
        self.lookup().await;
    }
    /// Count requests without returning their secret-bearing headers/bodies.
    ///
    /// # Panics
    /// Panics if the fixture did not retain its request history.
    pub async fn request_count(&self) -> usize {
        self.server
            .received_requests()
            .await
            .expect("recorded requests")
            .len()
    }
}
