//! Reuses C's integrated SDK token-file/KV2 wire protocol, with isolated endpoints.
use super::d_vendor::{must, RAW_KEY, SECRET_OIDC};
use oauth_connect_configuration::{
    OpenBaoAuthConfig, OpenBaoConfig, ProviderConfig, SecretReference, SecretsMode,
};
use serde_json::json;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

pub struct Vault {
    pub server: MockServer,
    token: tempfile::NamedTempFile,
}
impl Vault {
    pub async fn new() -> Self {
        let token = tempfile::NamedTempFile::new().expect("isolated vault token file");
        std::fs::write(token.path(), "bootstrap-service-sentinel")
            .expect("synthetic bootstrap credential");
        let result = Self {
            server: MockServer::start().await,
            token,
        };
        result.values(SECRET_OIDC, RAW_KEY).await;
        result
    }
    pub fn config(&self) -> OpenBaoConfig {
        OpenBaoConfig {
            endpoint: self.server.uri(),
            auth: OpenBaoAuthConfig::TokenFile {
                path: self.token.path().to_owned(),
            },
            allow_loopback_http: true,
            ca_cert_file: None,
        }
    }
    pub async fn values(&self, client: &str, hmac: &str) {
        for (route, value) in [
            (
                "/v1/auth/token/lookup-self",
                json!({"data": {"ttl": 600, "policies": ["connector-read"]}}),
            ),
            (
                "/v1/secret/data/service/oauth",
                json!({"data": {"data": {"client": client}}}),
            ),
            (
                "/v1/secret/data/service/hmac",
                json!({"data": {"data": {"key": hmac}}}),
            ),
        ] {
            Mock::given(method("GET"))
                .and(path(route))
                .and(header("X-Vault-Token", "bootstrap-service-sentinel"))
                .respond_with(ResponseTemplate::new(200).set_body_json(value))
                .mount(&self.server)
                .await;
        }
    }
    pub fn provider(p: &mut ProviderConfig) {
        p.secrets_mode = SecretsMode::Openbao;
        p.client_secret = None;
        p.hmac_secret = None;
        p.client_secret_ref = Some(must(
            SecretReference::new("secret".into(), "service/oauth".into(), "client".into()),
            "SDK client reference",
        ));
        p.hmac_secret_ref = Some(must(
            SecretReference::new("secret".into(), "service/hmac".into(), "key".into()),
            "SDK key reference",
        ));
    }
    pub async fn reads(&self) -> usize {
        self.server
            .received_requests()
            .await
            .expect("isolated vault receipts")
            .len()
    }
    pub async fn initial_reads(&self) {
        let receipts = self
            .server
            .received_requests()
            .await
            .expect("isolated vault receipts");
        for route in [
            "/v1/secret/data/service/oauth",
            "/v1/secret/data/service/hmac",
        ] {
            assert_eq!(
                receipts
                    .iter()
                    .filter(|r| r.url.path() == route && r.method.as_str() == "GET")
                    .count(),
                1,
                "each configured KV value is acquired once at boot"
            );
        }
    }
    pub async fn denied_read(&self) {
        let receipts = self
            .server
            .received_requests()
            .await
            .expect("isolated vault receipts");
        assert_eq!(
            receipts
                .iter()
                .filter(|r| r.url.path() == "/v1/secret/data/service/oauth"
                    && r.method.as_str() == "GET")
                .count(),
            1,
            "the rejected KV acquisition must actually be reached once"
        );
    }
}
