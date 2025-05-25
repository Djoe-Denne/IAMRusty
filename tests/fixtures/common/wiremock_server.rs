use wiremock::MockServer;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Shared wiremock server instance for all fixtures
static MOCK_SERVER: OnceCell<Arc<MockServer>> = OnceCell::const_new();

/// Get or create the shared mock server instance
pub async fn get_mock_server() -> Arc<MockServer> {
    MOCK_SERVER
        .get_or_init(|| async {
            Arc::new(MockServer::start().await)
        })
        .await
        .clone()
}

/// Get the base URL for the mock server
pub async fn get_mock_base_url() -> String {
    let server = get_mock_server().await;
    server.uri()
} 