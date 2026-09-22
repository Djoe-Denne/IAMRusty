//! `OpenBao` Transit T6 — engine Transit, token distinct, pas un mount KV.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::Client;
use testcontainers::core::ContainerPort;
use testcontainers::{ContainerAsync, GenericImage, ImageExt, runners::AsyncRunner};
use tokio::sync::Mutex;

use super::{NETWORK_NAME, docker_rm};

/// Image `OpenBao` (même tag que le compose plateforme, autre conteneur).
pub const IMAGE: &str = "openbao/openbao";
/// Tag `OpenBao` piné.
pub const TAG: &str = "2.6.2";
/// Nom Docker unique.
pub const CONTAINER_NAME: &str = "apparatus_operator_test-openbao-transit";
/// Token root de dev (distinct du token Lazaret).
pub const ROOT_TOKEN: &str = "apparatus-operator-transit-dev";
/// Nom de clé Transit pour Cosign (ECDSA P-256).
pub const TRANSIT_KEY: &str = "apparatus-p4-cosign";
const CONTAINER_PORT: u16 = 8200;
const READY_ATTEMPTS: u32 = 60;
const READY_SLEEP: Duration = Duration::from_millis(200);

static TEST_TRANSIT_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestOpenBaoTransitContainer>>>>> =
    OnceLock::new();
static TRANSIT_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Handle du conteneur Transit. `_container` est retenu pour la durée du process.
struct TestOpenBaoTransitContainer {
    _container: ContainerAsync<GenericImage>,
    port: u16,
}

/// Fixture `OpenBao` Transit typée.
pub struct TestOpenBaoTransit {
    /// Port hôte.
    pub port: u16,
    client: Client,
}

impl TestOpenBaoTransit {
    /// Démarre ou réutilise le singleton, monte Transit, crée la clé ECDSA.
    ///
    /// # Errors
    ///
    /// Échoue si Docker ne peut pas démarrer/pull `OpenBao`, si `/v1/sys/health`
    /// ne répond pas, ou si le mount Transit / la clé échoue.
    pub async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let port = get_or_create_test_transit_container().await?;
        let fixture = Arc::new(Self {
            port,
            client: Client::builder().timeout(Duration::from_secs(5)).build()?,
        });
        fixture.wait_for_ready().await?;
        fixture.ensure_transit_key().await?;
        Ok(fixture)
    }

    /// `http://127.0.0.1:{port}` pour reqwest hôte.
    #[must_use]
    pub fn host_base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// `http://{container}:8200` pour Cosign dans le réseau Docker.
    #[must_use]
    pub fn network_base_url() -> String {
        format!("http://{CONTAINER_NAME}:{CONTAINER_PORT}")
    }

    /// Token de dev.
    #[must_use]
    pub const fn token() -> &'static str {
        ROOT_TOKEN
    }

    /// Nom de la clé Transit.
    #[must_use]
    pub const fn key_name() -> &'static str {
        TRANSIT_KEY
    }

    async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/v1/sys/health", self.host_base_url());
        for _ in 0..READY_ATTEMPTS {
            match self.client.get(&url).send().await {
                Ok(response) if matches!(response.status().as_u16(), 200 | 429 | 472 | 473) => {
                    return Ok(());
                }
                _ => tokio::time::sleep(READY_SLEEP).await,
            }
        }
        Err(
            format!("OpenBao Transit readiness timeout on {url} (Docker image pull/start failed?)")
                .into(),
        )
    }

    async fn ensure_transit_key(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mounts = format!("{}/v1/sys/mounts/transit", self.host_base_url());
        let mount_resp = self
            .client
            .post(&mounts)
            .header("X-Vault-Token", Self::token())
            .header("Content-Type", "application/json")
            .body(r#"{"type":"transit"}"#)
            .send()
            .await?;
        let mount_status = mount_resp.status().as_u16();
        if !mount_resp.status().is_success() && mount_status != 400 {
            let text = mount_resp.text().await.unwrap_or_default();
            return Err(format!("enable Transit failed: {mount_status} {text}").into());
        }

        let key_url = format!("{}/v1/transit/keys/{TRANSIT_KEY}", self.host_base_url());
        let create = self
            .client
            .post(&key_url)
            .header("X-Vault-Token", Self::token())
            .header("Content-Type", "application/json")
            .body(r#"{"type":"ecdsa-p256"}"#)
            .send()
            .await?;
        let create_status = create.status().as_u16();
        if !create.status().is_success() && create_status != 400 {
            let text = create.text().await.unwrap_or_default();
            return Err(format!("create Transit key failed: {create_status} {text}").into());
        }
        Ok(())
    }
}

async fn get_or_create_test_transit_container() -> Result<u16, Box<dyn std::error::Error>> {
    let container_mutex = TEST_TRANSIT_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut container_guard = container_mutex.lock().await;
    if let Some(ref container) = *container_guard {
        return Ok(container.port);
    }

    docker_rm(CONTAINER_NAME);

    let image = GenericImage::new(IMAGE, TAG)
        .with_exposed_port(ContainerPort::Tcp(CONTAINER_PORT))
        .with_container_name(CONTAINER_NAME)
        .with_network(NETWORK_NAME)
        .with_startup_timeout(Duration::from_secs(180))
        .with_cap_add("IPC_LOCK")
        .with_env_var("BAO_DEV_LISTEN_ADDRESS", "0.0.0.0:8200")
        .with_env_var("VAULT_DEV_LISTEN_ADDRESS", "0.0.0.0:8200")
        .with_env_var("BAO_DEV_ROOT_TOKEN_ID", ROOT_TOKEN)
        .with_env_var("VAULT_DEV_ROOT_TOKEN_ID", ROOT_TOKEN)
        .with_cmd([
            "server",
            "-dev",
            "-dev-listen-address=0.0.0.0:8200",
            "-dev-root-token-id=apparatus-operator-transit-dev",
        ]);

    let container = image.start().await.map_err(|err| {
        format!(
            "Docker cannot start {CONTAINER_NAME} ({IMAGE}:{TAG}): {err}. Docker daemon must be running; image pull errors must not be ignored."
        )
    })?;
    let port = container
        .get_host_port_ipv4(ContainerPort::Tcp(CONTAINER_PORT))
        .await
        .map_err(|err| format!("Docker cannot map OpenBao Transit port: {err}"))?;
    *container_guard = Some(Arc::new(TestOpenBaoTransitContainer {
        _container: container,
        port,
    }));
    drop(container_guard);
    register_transit_cleanup_handler();
    Ok(port)
}

fn register_transit_cleanup_handler() {
    let _ = TRANSIT_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst);
}
