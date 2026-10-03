//! Redis testcontainer for Lazaret KV adapter tests.
//!
//! Container name: `lazaret_test-redis`. Service-local; not `has_redis` on the shared descriptor.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

static TEST_REDIS_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestRedisContainer>>>>> =
    OnceLock::new();
static REDIS_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Owns the `ContainerAsync`.
pub struct TestRedisContainer {
    container: ContainerAsync<GenericImage>,
    /// Mapped host port.
    pub port: u16,
}

impl TestRedisContainer {
    /// Stop and remove.
    pub async fn cleanup(self) {
        info!("Stopping Lazaret test Redis container");
        if let Err(e) = self.container.stop().await {
            warn!("Failed to stop Redis: {e}");
        }
        if let Err(e) = self.container.rm().await {
            warn!("Failed to remove Redis: {e}");
        }
    }
}

/// Typed Redis fixture. Tests use [`Self::url`].
pub struct TestRedis {
    /// Host port mapped to 6379.
    pub port: u16,
}

impl TestRedis {
    /// Start or reuse the singleton container.
    ///
    /// # Errors
    ///
    /// Returns an error if Docker cannot start Redis or PING never succeeds.
    pub async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let port = get_or_create_test_redis_container().await?;
        let fixture = Arc::new(Self { port });
        fixture.wait_for_ready().await?;
        Ok(fixture)
    }

    /// `redis://127.0.0.1:{port}`
    #[must_use]
    pub fn url(&self) -> String {
        format!("redis://127.0.0.1:{}", self.port)
    }

    async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = self.url();
        for _ in 0..40 {
            if let Ok(client) = redis::Client::open(url.as_str()) {
                if let Ok(mut con) = client.get_connection() {
                    let ping: Result<String, _> = redis::cmd("PING").query(&mut con);
                    if ping.ok().as_deref() == Some("PONG") {
                        debug!("Redis is ready");
                        return Ok(());
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err("redis readiness timeout".into())
    }

    /// FLUSHDB for isolation between tests.
    ///
    /// # Errors
    ///
    /// Returns an error if FLUSHDB fails.
    pub fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = redis::Client::open(self.url())?;
        let mut con = client.get_connection()?;
        redis::cmd("FLUSHDB").query::<()>(&mut con)?;
        Ok(())
    }
}

async fn get_or_create_test_redis_container() -> Result<u16, Box<dyn std::error::Error>> {
    let container_mutex = TEST_REDIS_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut container_guard = container_mutex.lock().await;
    if let Some(ref container) = *container_guard {
        return Ok(container.port);
    }

    cleanup_existing_redis_container();

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let image = GenericImage::new("redis", "7-alpine")
        .with_container_name("lazaret_test-redis")
        .with_mapped_port(port, testcontainers::core::ContainerPort::Tcp(6379));

    info!("Starting Redis container on port {port}");
    let container = image.start().await?;
    let test_container = Arc::new(TestRedisContainer { container, port });
    *container_guard = Some(test_container);
    register_redis_cleanup_handler();
    Ok(port)
}

fn cleanup_existing_redis_container() {
    use std::process::Command;
    debug!("Checking for existing lazaret_test-redis");
    let _ = Command::new("docker")
        .args(["stop", "lazaret_test-redis"])
        .output();
    let _ = Command::new("docker")
        .args(["rm", "-f", "lazaret_test-redis"])
        .output();
}

fn register_redis_cleanup_handler() {
    if REDIS_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    info!("Registering Redis test container cleanup handler");
}
