//! Registry zot T6 : artifacts OCI + pull. Seul le signer pousse.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::Client;
use testcontainers::core::ContainerPort;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use tokio::sync::Mutex;

use super::{docker_rm, NETWORK_NAME};

/// Image zot (tag stable). Artifacts + pull d'image.
pub const IMAGE: &str = "ghcr.io/project-zot/zot";
/// Tag zot piné.
pub const TAG: &str = "v2.1.8";
/// Nom Docker unique (pas un nom Lazaret).
pub const CONTAINER_NAME: &str = "apparatus_operator_test-zot";
/// Identité autorisée à pousser.
pub const SIGNER_USER: &str = "signer";
/// Mot de passe htpasswd de test (pas un secret de production).
pub const SIGNER_PASSWORD: &str = "apparatus-signer-push";
const CONTAINER_PORT: u16 = 5000;
const READY_ATTEMPTS: u32 = 60;
const READY_SLEEP: Duration = Duration::from_millis(250);

static TEST_ZOT_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestZotContainer>>>>> = OnceLock::new();
static ZOT_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Handle du conteneur zot. `container` est retenu pour la durée du process.
struct TestZotContainer {
    _container: ContainerAsync<GenericImage>,
    port: u16,
}

/// Fixture zot typée pour les tests T6.
pub struct TestZot {
    /// Port hôte.
    pub port: u16,
    client: Client,
}

impl TestZot {
    /// Démarre ou réutilise le singleton zot.
    ///
    /// # Errors
    ///
    /// Échoue si Docker ne peut pas démarrer/pull zot, ou si `/v2/` ne répond
    /// pas avant le délai.
    pub async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let port = get_or_create_test_zot_container().await?;
        let fixture = Arc::new(Self {
            port,
            client: Client::builder().timeout(Duration::from_secs(5)).build()?,
        });
        fixture.wait_for_ready().await?;
        Ok(fixture)
    }

    /// `http://127.0.0.1:{port}` pour le process hôte.
    #[must_use]
    pub fn host_base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Registry vue depuis le réseau Docker (helpers ORAS/Cosign).
    #[must_use]
    pub fn network_registry() -> String {
        format!("{CONTAINER_NAME}:{CONTAINER_PORT}")
    }

    async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/v2/", self.host_base_url());
        for _ in 0..READY_ATTEMPTS {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    let code = response.status().as_u16();
                    if (200..500).contains(&code) {
                        return Ok(());
                    }
                }
                Err(_) => tokio::time::sleep(READY_SLEEP).await,
            }
            tokio::time::sleep(READY_SLEEP).await;
        }
        Err(format!("zot readiness timeout on {url} (Docker image pull/start failed?)").into())
    }
}

async fn get_or_create_test_zot_container() -> Result<u16, Box<dyn std::error::Error>> {
    let container_mutex = TEST_ZOT_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut container_guard = container_mutex.lock().await;
    if let Some(ref container) = *container_guard {
        return Ok(container.port);
    }

    docker_rm(CONTAINER_NAME);

    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zot");
    let image = GenericImage::new(IMAGE, TAG)
        .with_exposed_port(ContainerPort::Tcp(CONTAINER_PORT))
        .with_container_name(CONTAINER_NAME)
        .with_network(NETWORK_NAME)
        .with_startup_timeout(Duration::from_secs(180))
        .with_copy_to("/etc/zot/config.json", fixture_dir.join("config.json"))
        .with_copy_to("/etc/zot/htpasswd", fixture_dir.join("htpasswd"))
        .with_cmd(["serve", "/etc/zot/config.json"]);

    let container = image.start().await.map_err(|err| {
        format!(
            "Docker cannot start {CONTAINER_NAME} ({IMAGE}:{TAG}): {err}. Docker daemon must be running; image pull errors must not be ignored."
        )
    })?;
    let port = container
        .get_host_port_ipv4(ContainerPort::Tcp(CONTAINER_PORT))
        .await
        .map_err(|err| format!("Docker cannot map zot port: {err}"))?;
    *container_guard = Some(Arc::new(TestZotContainer {
        _container: container,
        port,
    }));
    drop(container_guard);
    register_zot_cleanup_handler();
    Ok(port)
}

fn register_zot_cleanup_handler() {
    let _ = ZOT_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst);
}
