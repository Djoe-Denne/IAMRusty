//! SQS LocalStack test container utilities
//!
//! This module provides a LocalStack SQS container for integration tests to verify real
//! event publishing functionality.

use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_sqs::{types::Message, Client, Config};
use rustycog_config::{load_config_part, QueueConfig, SqsConfig};
use rustycog_core::error::ServiceError;
use rustycog_events::event::DomainEvent;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid;

/// Global test SQS container instance
static TEST_SQS_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestSqsContainer>>>>> = OnceLock::new();

/// Flag to track if cleanup handler has been registered
static SQS_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Test SQS container wrapper
pub struct TestSqsContainer {
    container: ContainerAsync<GenericImage>,
    pub endpoint_url: String,
    pub port: u16,
}

impl TestSqsContainer {
    /// Stop and remove the container
    pub async fn cleanup(self) {
        info!("Stopping and removing test SQS LocalStack container");
        if let Err(e) = self.container.stop().await {
            warn!("Failed to stop SQS container: {}", e);
        } else {
            info!("SQS container stopped successfully");
        }
        if let Err(e) = self.container.rm().await {
            warn!("Failed to remove SQS container: {}", e);
        } else {
            info!("SQS container removed successfully");
        }
        info!("Test SQS container cleanup completed");
    }
}

/// Test SQS fixture providing SQS connection and utilities
pub struct TestSqs {
    pub client: Client,
    pub endpoint_url: String,
    pub queue_url: String,
    pub region: String,
}

impl TestSqs {
    /// Get or create the global test SQS instance
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (_container, sqs_config) = get_or_create_test_sqs_container().await?;
        let host = sqs_config.host.clone();
        let port = sqs_config.actual_port();
        let region = sqs_config.region.clone();

        // Parse the endpoint URL to get host and port
        let endpoint_url = sqs_config
            .endpoint_url()
            .unwrap_or("http://localhost:4566".to_string());

        let access_key_id = sqs_config
            .access_key_id
            .clone()
            .unwrap_or("test".to_string());
        let secret_access_key = sqs_config
            .secret_access_key
            .clone()
            .unwrap_or("test".to_string());
        let account_id = sqs_config.account_id.clone();
        let default_queue = sqs_config.default_queue.clone();

        // Set environment variables for SQS configuration so our app config picks it up
        unsafe {
            // Configure queue type to SQS
            std::env::set_var("IAM_QUEUE__TYPE", "sqs");
            // Configure SQS-specific settings
            std::env::set_var("IAM_QUEUE__SQS__HOST", host);
            std::env::set_var("IAM_QUEUE__SQS__PORT", &port.to_string());
            std::env::set_var("IAM_QUEUE__SQS__ENABLED", "true");
            std::env::set_var("IAM_QUEUE__SQS__REGION", &region);
            std::env::set_var("IAM_QUEUE__SQS__ACCESS_KEY_ID", access_key_id);
            std::env::set_var("IAM_QUEUE__SQS__SECRET_ACCESS_KEY", secret_access_key);
            std::env::set_var("IAM_QUEUE__SQS__ACCOUNT_ID", &account_id); // LocalStack default
            std::env::set_var("IAM_QUEUE__SQS__DEFAULT_QUEUE", &default_queue);
        }

        // Create SQS client
        let client = Self::create_sqs_client(&endpoint_url, &region).await?;

        // Wait for LocalStack to be ready
        Self::wait_for_localstack(&endpoint_url).await?;

        // Create test queue using configured queue name
        let queue_url = Self::create_test_queue(&client, &sqs_config).await?;

        Ok(Self {
            client,
            endpoint_url,
            queue_url,
            region,
        })
    }

    /// Create SQS client configured for LocalStack
    async fn create_sqs_client(
        endpoint_url: &str,
        region: &str,
    ) -> Result<Client, Box<dyn std::error::Error>> {
        // Create credentials for LocalStack
        let credentials = Credentials::new("test", "test", None, None, "rustycog-test");

        // Configure AWS SDK for LocalStack
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .endpoint_url(endpoint_url)
            .credentials_provider(credentials)
            .load()
            .await;

        let sqs_config = Config::from(&aws_config);
        let client = Client::from_conf(sqs_config);

        Ok(client)
    }

    /// Wait for LocalStack to be ready
    async fn wait_for_localstack(endpoint_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Waiting for LocalStack to be ready...");

        let max_attempts = 30;
        let mut attempts = 0;

        // Extract host and port from endpoint URL
        let url = url::Url::parse(endpoint_url)?;
        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(4566);

        while attempts < max_attempts {
            // Try to connect to LocalStack
            match tokio::net::TcpStream::connect((host, port)).await {
                Ok(_) => {
                    info!("LocalStack is ready after {} attempts", attempts + 1);
                    // Give it a moment more to fully initialize SQS service
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    return Ok(());
                }
                Err(e) => {
                    debug!("LocalStack connection failed: {}", e);
                }
            }

            attempts += 1;
            if attempts < max_attempts {
                debug!(
                    "Retrying LocalStack connection in 1 second... (attempt {}/{})",
                    attempts, max_attempts
                );
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        Err(format!(
            "LocalStack failed to become ready after {} attempts",
            max_attempts
        )
        .into())
    }

    /// Create test queue using the queue name from configuration
    async fn create_test_queue(
        client: &Client,
        sqs_config: &SqsConfig,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Get the queue name from the actual configuration
        let queue_name = &sqs_config.default_queue;

        debug!("Creating test queue with configured name: {}", queue_name);

        let result = client.create_queue().queue_name(queue_name).send().await?;

        let queue_url = result.queue_url().unwrap_or_default().to_string();
        info!("Created test queue: {}", queue_url);

        Ok(queue_url)
    }

    /// Send a test message to the queue (raw string)
    pub async fn send_message(
        &self,
        message_body: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        debug!("Sending message to queue: {}", self.queue_url);

        let result = self
            .client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(message_body)
            .send()
            .await?;

        let message_id = result.message_id().unwrap_or("unknown").to_string();
        debug!("Message sent with ID: {}", message_id);

        Ok(message_id)
    }

    /// Send a domain event to the queue (formatted like the SQS publisher)
    pub async fn send_event(
        &self,
        event: &dyn DomainEvent,
    ) -> Result<String, Box<dyn std::error::Error>> {
        info!("Sending domain event to queue: {}", self.queue_url);

        // Format the event the same way the SQS publisher does
        let message_body = self.serialize_event(event)?;

        let result = self
            .client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(message_body)
            .send()
            .await?;

        let message_id = result.message_id().unwrap_or("unknown").to_string();
        info!("Event sent with ID: {}", message_id);

        Ok(message_id)
    }

    /// Serialize domain event to SQS message body (same as SQS publisher)
    fn serialize_event(
        &self,
        event: &dyn DomainEvent,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Get the event JSON and parse it back to a Value so it's properly structured in the data field
        let event_json_str = event
            .to_json()
            .map_err(|e| format!("Failed to get event JSON: {}", e))?;
        let event_data: serde_json::Value = serde_json::from_str(&event_json_str)
            .map_err(|e| format!("Failed to parse event JSON: {}", e))?;

        let message_body = json!({
            "event_id": event.event_id(),
            "event_type": event.event_type(),
            "aggregate_id": event.aggregate_id(),
            "occurred_at": event.occurred_at(),
            "version": event.version(),
            "data": event_data,
            "metadata": event.metadata()
        });

        serde_json::to_string(&message_body)
            .map_err(|e| format!("Failed to serialize event for SQS: {}", e).into())
    }

    /// Receive messages from the queue
    pub async fn receive_messages(
        &self,
        max_messages: i32,
        wait_time_seconds: i32,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        debug!("Receiving messages from queue: {}", self.queue_url);

        let result = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(max_messages)
            .wait_time_seconds(wait_time_seconds)
            .send()
            .await?;

        let messages = result.messages().to_vec();
        debug!("Received {} messages", messages.len());

        Ok(messages)
    }

    /// Delete a message from the queue
    pub async fn delete_message(
        &self,
        receipt_handle: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Deleting message from queue: {}", self.queue_url);

        self.client
            .delete_message()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await?;

        debug!("Message deleted successfully");
        Ok(())
    }

    /// Get all messages from the queue (non-destructive polling)
    pub async fn get_all_messages(
        &self,
        max_wait_secs: u64,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        debug!(
            "Getting all messages from queue with max wait: {} seconds",
            max_wait_secs
        );

        let mut all_messages = Vec::new();
        let start_time = std::time::Instant::now();

        while start_time.elapsed().as_secs() < max_wait_secs {
            let messages = self.receive_messages(10, 1).await?;

            if messages.is_empty() {
                // No messages, wait a bit before trying again
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            for message in messages {
                if let Some(body) = message.body() {
                    all_messages.push(body.to_string());
                }

                // Delete the message to avoid reprocessing
                if let Some(receipt_handle) = message.receipt_handle() {
                    if let Err(e) = self.delete_message(receipt_handle).await {
                        warn!("Failed to delete message: {}", e);
                    }
                }
            }
        }

        debug!("Retrieved {} messages total", all_messages.len());
        Ok(all_messages)
    }

    /// Wait for a specific number of messages to be available
    pub async fn wait_for_messages(
        &self,
        expected_count: usize,
        max_wait_secs: u64,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        debug!(
            "Waiting for {} messages with max wait: {} seconds",
            expected_count, max_wait_secs
        );

        let mut all_messages = Vec::new();
        let start_time = std::time::Instant::now();

        while all_messages.len() < expected_count && start_time.elapsed().as_secs() < max_wait_secs
        {
            let messages = self.receive_messages(10, 2).await?;

            for message in messages {
                if let Some(body) = message.body() {
                    all_messages.push(body.to_string());
                }

                // Delete the message to avoid reprocessing
                if let Some(receipt_handle) = message.receipt_handle() {
                    if let Err(e) = self.delete_message(receipt_handle).await {
                        warn!("Failed to delete message: {}", e);
                    }
                }

                if all_messages.len() >= expected_count {
                    break;
                }
            }

            if all_messages.len() < expected_count {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        if all_messages.len() < expected_count {
            return Err(format!(
                "Only received {} out of {} expected messages within {} seconds",
                all_messages.len(),
                expected_count,
                max_wait_secs
            )
            .into());
        }

        debug!("Successfully received {} messages", all_messages.len());
        Ok(all_messages)
    }

    /// Purge all messages from the queue
    pub async fn purge_queue(&self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Purging queue: {}", self.queue_url);

        self.client
            .purge_queue()
            .queue_url(&self.queue_url)
            .send()
            .await?;

        info!("Queue purged successfully");
        Ok(())
    }
}

/// Get or create the global test SQS container
async fn get_or_create_test_sqs_container(
) -> Result<(Arc<TestSqsContainer>, SqsConfig), Box<dyn std::error::Error>> {
    let container_mutex = TEST_SQS_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));

    let mut container_guard = container_mutex.lock().await;

    if let Some(ref container) = *container_guard {
        // If container exists, we still need to load the config to return it
        let queue_config =
            load_config_part::<QueueConfig>("queue").expect("failed to load queue config");
        let sqs_config = match &queue_config {
            QueueConfig::Sqs(sqs_config) => sqs_config.clone(),
            QueueConfig::Kafka(_) => {
                return Err("Configuration is set to Kafka, but SQS test container requires SQS configuration. Environment variables may not be set correctly.".into());
            }
            QueueConfig::Disabled => {
                return Err("Queue is disabled, but SQS test container requires SQS configuration. Environment variables may not be set correctly.".into());
            }
        };
        return Ok((container.clone(), sqs_config));
    }

    info!("Creating new SQS LocalStack test container");

    // Clean up any existing container
    cleanup_existing_sqs_container().await;

    // Clear only the SQS port cache to ensure fresh random port generation
    SqsConfig::clear_port_cache();

    // Load configuration to understand SQS settings
    let queue_config =
        load_config_part::<QueueConfig>("queue").expect("failed to load queue config");
    let sqs_config = match &queue_config {
        QueueConfig::Sqs(sqs_config) => sqs_config.clone(),
        QueueConfig::Kafka(_) => {
            return Err("Configuration is set to Kafka, but SQS test container requires SQS configuration. Environment variables may not be set correctly.".into());
        }
        QueueConfig::Disabled => {
            return Err("Queue is disabled, but SQS test container requires SQS configuration. Environment variables may not be set correctly.".into());
        }
    };

    // Use the configuration's port resolution mechanism
    let sqs_port = sqs_config.actual_port();

    // Create LocalStack container with SQS service
    let localstack_image = GenericImage::new("localstack/localstack", "3.0.2")
        .with_env_var("SERVICES", "sqs")
        .with_env_var("DEBUG", "1")
        .with_env_var("DATA_DIR", "/tmp/localstack/data")
        .with_env_var("DOCKER_HOST", "unix:///var/run/docker.sock")
        .with_env_var("HOST_TMP_FOLDER", "/tmp")
        .with_container_name("iam_test-localstack-sqs")
        .with_mapped_port(sqs_port, testcontainers::core::ContainerPort::Tcp(4566)); // LocalStack default port

    // Start LocalStack
    info!("Starting LocalStack SQS container on port {}...", sqs_port);
    let sqs_container = localstack_image.start().await?;

    let endpoint_url = format!("http://localhost:{}", sqs_port);

    info!("Test SQS LocalStack container started");
    info!("Endpoint URL: {}", endpoint_url);

    let test_container = Arc::new(TestSqsContainer {
        container: sqs_container,
        endpoint_url,
        port: sqs_port,
    });

    *container_guard = Some(test_container.clone());

    // Register cleanup handler on first container creation
    register_sqs_cleanup_handler().await;

    Ok((test_container, sqs_config))
}

/// Clean up any existing SQS containers
async fn cleanup_existing_sqs_container() {
    use std::process::Command;

    debug!("Checking for existing SQS LocalStack test containers");

    let containers = ["iam_test-localstack-sqs"];

    for container_name in &containers {
        // Stop the container
        let _ = Command::new("docker")
            .args(&["stop", container_name])
            .output();

        // Remove the container
        let _ = Command::new("docker")
            .args(&["rm", "-f", container_name])
            .output();

        debug!("Cleaned up container: {}", container_name);
    }
}

/// Register cleanup handler for SQS containers
async fn register_sqs_cleanup_handler() {
    if SQS_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }

    info!("Registering SQS test container cleanup handler");
}

/// Test fixture for SQS integration tests
pub struct TestSqsFixture {
    pub sqs: TestSqs,
}

impl TestSqsFixture {
    /// Create a new SQS test fixture
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sqs = TestSqs::new().await?;
        Ok(Self { sqs })
    }

    /// Wait for and verify a specific event was published
    pub async fn verify_event_published(
        &self,
        event_type: &str,
        timeout_secs: u64,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let messages = self.sqs.get_all_messages(timeout_secs).await?;

        for message in messages {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&message) {
                if let Some(event_type_value) = event.get("event_type") {
                    if event_type_value == event_type {
                        return Ok(event);
                    }
                }
            }
        }

        Err(format!(
            "Event with type '{}' not found within {} seconds",
            event_type, timeout_secs
        )
        .into())
    }

    /// Send a test event to the queue
    pub async fn send_test_event(
        &self,
        event_type: &str,
        aggregate_id: &str,
        data: Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let event = serde_json::json!({
            "event_id": uuid::Uuid::new_v4().to_string(),
            "event_type": event_type,
            "aggregate_id": aggregate_id,
            "occurred_at": chrono::Utc::now().to_rfc3339(),
            "version": 1,
            "data": data,
            "metadata": {}
        });

        self.sqs.send_message(&serde_json::to_string(&event)?).await
    }

    /// Cleanup SQS container (for test cleanup)
    pub async fn cleanup_container() -> Result<(), Box<dyn std::error::Error>> {
        let container_mutex = TEST_SQS_CONTAINER.get();
        if let Some(container_mutex) = container_mutex {
            let mut container_guard = container_mutex.lock().await;
            if let Some(container_arc) = container_guard.take() {
                info!("Manually cleaning up test SQS container");

                match Arc::try_unwrap(container_arc) {
                    Ok(container) => {
                        container.cleanup().await;
                        info!("Test SQS container cleanup completed");
                    }
                    Err(_) => {
                        warn!("Could not cleanup SQS container: still has references");
                        // Fallback cleanup using Docker commands
                        cleanup_existing_sqs_container().await;
                    }
                }
            }
        }
        Ok(())
    }
}
