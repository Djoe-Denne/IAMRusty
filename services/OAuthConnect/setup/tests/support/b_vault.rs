//! B-1/B-2 isolated outbound fixture, following legacy `OpenBaoService` routes/bootstrap.

use oauth_connect_configuration::{OpenBaoAuthConfig, OpenBaoConfig};
use serde_json::{json, Value};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Synthetic bootstrap credential, never a live token.
pub const TOKEN: &str = "bootstrap-token-sentinel";
/// SDK bootstrap phase.
pub const LOOKUP: &str = "/v1/auth/token/lookup-self";
/// Existing GitHub KV location remains unchanged during this refactor.
pub const CLIENT_PATH: &str = "/v1/secret/data/github-connect/oauth";
/// Existing GitHub IAM HMAC location, independent of the OAuth credential.
pub const HMAC_PATH: &str = "/v1/secret/data/github-connect/iam";

/// Owns a random-port `WireMock` and a real regular token file; no shared singleton/env.
pub struct Vault {
    /// Owns the listener and retained request journal for this test only.
    pub server: MockServer,
    /// Real regular bootstrap file, removed after fixture use.
    pub token: tempfile::NamedTempFile,
}

impl Vault {
    /// Bind an isolated server. Call lookup explicitly so failures never overlap stubs.
    pub async fn start() -> Self {
        let token = tempfile::NamedTempFile::new().expect("isolated token file");
        std::fs::write(token.path(), TOKEN).expect("synthetic token file");
        Self {
            server: MockServer::start().await,
            token,
        }
    }

    /// Use the actual SDK loopback exception and explicit token-file authentication.
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

    /// Install the lookup phase separately from KV acquisition.
    pub async fn lookup(&self, ttl: u64, policies: &[&str]) {
        self.response(
            LOOKUP,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"ttl": ttl, "policies": policies}})),
        )
        .await;
    }

    /// Route and authenticate every response exactly; an unmounted path stays a failure.
    pub async fn response(&self, endpoint: &str, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .and(header("X-Vault-Token", TOKEN))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }

    /// SDK KV-v2 wire representation, not a fake resolver or a fake service result.
    pub async fn secret(&self, endpoint: &str, field: &str, value: &str) {
        self.response(
            endpoint,
            ResponseTemplate::new(200).set_body_json(json!({"data": {"data": {field: value}}})),
        )
        .await;
    }

    /// Count only a named GET phase, never expose token-bearing request objects to asserts.
    pub async fn count(&self, endpoint: &str) -> usize {
        self.server
            .received_requests()
            .await
            .expect("retained vault receipts")
            .iter()
            .filter(|r| r.method.as_str() == "GET" && r.url.path() == endpoint)
            .count()
    }

    /// Prove no network before prevalidation/file failure (includes unexpected paths).
    pub async fn total(&self) -> usize {
        self.server
            .received_requests()
            .await
            .expect("retained vault receipts")
            .len()
    }
}

/// Build raw JSON at a known inclusive byte boundary; never use a compression dependency.
pub fn padded_kv(size: usize) -> Vec<u8> {
    let value: Value = json!({"data": {"data": {"client_secret": "vault-client-sentinel"}}});
    let mut bytes = serde_json::to_vec(&value).expect("known tiny JSON");
    assert!(bytes.len() <= size, "fixture boundary exceeds base JSON");
    bytes.resize(size, b' ');
    bytes
}
