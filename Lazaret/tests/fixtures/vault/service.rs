//! Wiremock fake for Vault/OpenBao KV HTTP API.

use std::sync::Arc;

use rustycog::testing::wiremock::MockServerFixture;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::resources::VaultKvReadBody;

/// Wiremock Vault KV v2.
pub struct VaultMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl VaultMockService {
    /// Isolated listener.
    pub async fn new() -> Self {
        let fixture = MockServerFixture::isolated().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
        }
    }

    /// Base URL for [`lazaret_infra::VaultHttpSecretResolver`].
    #[must_use]
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Stub a successful KV v2 read.
    pub async fn mock_kv_read(
        &self,
        mount: &str,
        secret_path: &str,
        field: &str,
        value: &str,
    ) -> &Self {
        let route = format!("/v1/{mount}/data/{secret_path}");
        Mock::given(method("GET"))
            .and(path(route.as_str()))
            .and(header("X-Vault-Token", "test-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(VaultKvReadBody::one_field(field, value))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Wipe stubs.
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }
}
