//! OpenBao testcontainer for Lazaret secret-resolver product proof (T12).
//!
//! Container name: `lazaret_test-openbao`. Service-local; not `has_openbao` on
//! the shared descriptor. Same image tag as docker-compose (`openbao/openbao:2.6.2`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::Client;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

const IMAGE: &str = "openbao/openbao";
const TAG: &str = "2.6.2";
const CONTAINER_NAME: &str = "lazaret_test-openbao";
const ROOT_TOKEN: &str = "lazaret-dev-root";
const MOUNT: &str = "secret";
const CONTAINER_PORT: u16 = 8200;

static TEST_OPENBAO_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestOpenBaoContainer>>>>> =
    OnceLock::new();
static OPENBAO_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Owns the `ContainerAsync`.
pub struct TestOpenBaoContainer {
    container: ContainerAsync<GenericImage>,
    /// Mapped host port.
    pub port: u16,
}

impl TestOpenBaoContainer {
    /// Stop and remove.
    pub async fn cleanup(self) {
        info!("Stopping Lazaret test OpenBao container");
        if let Err(e) = self.container.stop().await {
            warn!("Failed to stop OpenBao: {e}");
        }
        if let Err(e) = self.container.rm().await {
            warn!("Failed to remove OpenBao: {e}");
        }
    }
}

/// Typed OpenBao fixture. Tests use [`Self::base_url`], [`Self::token`], and
/// [`Self::put_kv_v2`].
pub struct TestOpenBao {
    /// Host port mapped to 8200.
    pub port: u16,
    client: Client,
}

impl TestOpenBao {
    /// Start or reuse the singleton container.
    ///
    /// # Errors
    ///
    /// Returns an error if Docker cannot start OpenBao, the HTTP client cannot
    /// be built, or `/v1/sys/health` never reports ready.
    pub async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let port = get_or_create_test_openbao_container().await?;
        let fixture = Arc::new(Self {
            port,
            client: Client::builder().timeout(Duration::from_secs(5)).build()?,
        });
        fixture.wait_for_ready().await?;
        Ok(fixture)
    }

    /// `http://127.0.0.1:{port}` (no trailing slash).
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Dev root token (`lazaret-dev-root`), same class as compose.
    #[must_use]
    pub fn token(&self) -> &'static str {
        ROOT_TOKEN
    }

    /// KV v2 PUT `{mount}/data/{path}` with `{"data":{field:value}}`.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP PUT fails or the status is not success.
    pub async fn put_kv_v2(
        &self,
        path: &str,
        field: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let encoded_path = encode_vault_path(path);
        let url = format!("{}/v1/{MOUNT}/data/{encoded_path}", self.base_url());
        let body = format!(
            r#"{{"data":{{"{}":"{}"}}}}"#,
            json_escape(field),
            json_escape(value)
        );
        let response = self
            .client
            .put(&url)
            .header("X-Vault-Token", self.token())
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(format!("OpenBao KV v2 PUT {url} failed: {status} {text}").into());
        }
        Ok(())
    }

    async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/v1/sys/health", self.base_url());
        for _ in 0..60 {
            match self.client.get(&url).send().await {
                Ok(response) if matches!(response.status().as_u16(), 200 | 429) => {
                    debug!("OpenBao is ready");
                    return Ok(());
                }
                _ => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
        Err("openbao readiness timeout".into())
    }
}

async fn get_or_create_test_openbao_container() -> Result<u16, Box<dyn std::error::Error>> {
    let container_mutex = TEST_OPENBAO_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut container_guard = container_mutex.lock().await;
    if let Some(ref container) = *container_guard {
        return Ok(container.port);
    }

    cleanup_existing_openbao_container();

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let image = GenericImage::new(IMAGE, TAG)
        .with_container_name(CONTAINER_NAME)
        .with_mapped_port(
            port,
            testcontainers::core::ContainerPort::Tcp(CONTAINER_PORT),
        )
        .with_cap_add("IPC_LOCK")
        .with_env_var("BAO_DEV_LISTEN_ADDRESS", "0.0.0.0:8200")
        .with_env_var("VAULT_DEV_LISTEN_ADDRESS", "0.0.0.0:8200")
        .with_env_var("BAO_DEV_ROOT_TOKEN_ID", ROOT_TOKEN)
        .with_env_var("VAULT_DEV_ROOT_TOKEN_ID", ROOT_TOKEN)
        .with_cmd([
            "server",
            "-dev",
            "-dev-listen-address=0.0.0.0:8200",
            "-dev-root-token-id=lazaret-dev-root",
        ]);

    info!("Starting OpenBao container on port {port}");
    let container = image.start().await?;
    let test_container = Arc::new(TestOpenBaoContainer { container, port });
    *container_guard = Some(test_container);
    register_openbao_cleanup_handler();
    Ok(port)
}

fn cleanup_existing_openbao_container() {
    use std::process::Command;
    debug!("Checking for existing {CONTAINER_NAME}");
    let _ = Command::new("docker")
        .args(["stop", CONTAINER_NAME])
        .output();
    let _ = Command::new("docker")
        .args(["rm", "-f", CONTAINER_NAME])
        .output();
}

fn register_openbao_cleanup_handler() {
    if OPENBAO_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    info!("Registering OpenBao test container cleanup handler");
}

fn encode_vault_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
