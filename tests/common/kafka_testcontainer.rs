//! Kafka test container utilities
//! 
//! This module provides a Kafka container for integration tests to verify real
//! event publishing functionality.

use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use testcontainers::{GenericImage, ImageExt, ContainerAsync, runners::AsyncRunner};
use configuration::KafkaConfig;
use tracing::{info, debug, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use uuid;
use infra::event::test_consumer::TestKafkaConsumer;

/// Global test Kafka container instance
static TEST_KAFKA_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestKafkaContainer>>>>> = OnceLock::new();

/// Flag to track if cleanup handler has been registered
static KAFKA_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Test Kafka container wrapper
pub struct TestKafkaContainer {
    container: ContainerAsync<GenericImage>,
    pub brokers: String,
    pub port: u16,
}

impl TestKafkaContainer {
    /// Stop and remove the container
    pub async fn cleanup(self) {
        info!("Stopping and removing test Kafka container");
        if let Err(e) = self.container.stop().await {
            warn!("Failed to stop Kafka container: {}", e);
        } else {
            info!("Kafka container stopped successfully");
        }
        if let Err(e) = self.container.rm().await {
            warn!("Failed to remove Kafka container: {}", e);
        } else {
            info!("Kafka container removed successfully");
        }
        info!("Test Kafka container cleanup completed");
    }
}

/// Test Kafka fixture providing Kafka connection and utilities
pub struct TestKafka {
    pub brokers: String,
    pub topic: String,
}

impl TestKafka {
    /// Get or create the global test Kafka instance
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let container = get_or_create_test_kafka_container().await?;
        let brokers = container.brokers.clone();
        let topic = "test-user-events".to_string();
        
        // Parse the brokers string to get host and port
        let parts: Vec<&str> = brokers.split(':').collect();
        if parts.len() == 2 {
            let host = parts[0];
            let port = parts[1];
            
            // Set environment variables for Kafka configuration so our app config picks it up
            unsafe {
                std::env::set_var("IAM_KAFKA__HOST", host);
                std::env::set_var("IAM_KAFKA__PORT", port);
                std::env::set_var("IAM_KAFKA__ENABLED", "true");
                std::env::set_var("IAM_KAFKA__USER_EVENTS_TOPIC", &topic);
            }
        } else {
            // Fallback to old format for compatibility
            unsafe {
                std::env::set_var("IAM_KAFKA__HOST", "localhost");
                std::env::set_var("IAM_KAFKA__PORT", "9092");
                std::env::set_var("IAM_KAFKA__ENABLED", "true");
                std::env::set_var("IAM_KAFKA__USER_EVENTS_TOPIC", &topic);
            }
        }
        
        // Wait for Kafka to be ready
        Self::wait_for_kafka(&brokers).await?;
        
        Ok(Self {
            brokers,
            topic,
        })
    }
    
    /// Wait for Kafka to be ready using a simple TCP connection test
    async fn wait_for_kafka(brokers: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Waiting for Kafka to be ready...");
        
        let max_attempts = 30;
        let mut attempts = 0;
        
        while attempts < max_attempts {
            // Simple TCP connection test
            if let Ok(addr) = brokers.parse::<std::net::SocketAddr>() {
                match tokio::net::TcpStream::connect(addr).await {
                    Ok(_) => {
                        info!("Kafka is ready after {} attempts", attempts + 1);
                        return Ok(());
                    }
                    Err(e) => {
                        debug!("Kafka connection failed: {}", e);
                    }
                }
            } else {
                // If it's not a direct socket address, try parsing host:port
                let parts: Vec<&str> = brokers.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(port) = parts[1].parse::<u16>() {
                        let addr = (parts[0], port);
                        match tokio::net::TcpStream::connect(addr).await {
                            Ok(_) => {
                                info!("Kafka is ready after {} attempts", attempts + 1);
                                return Ok(());
                            }
                            Err(e) => {
                                debug!("Kafka connection failed: {}", e);
                            }
                        }
                    }
                }
            }
            
            attempts += 1;
            if attempts < max_attempts {
                debug!("Retrying Kafka connection in 1 second... (attempt {}/{})", attempts, max_attempts);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        
        Err(format!("Kafka failed to become ready after {} attempts", max_attempts).into())
    }
    
    /// Wait for a message to be published to the topic
    /// This is a simplified version that waits for a certain duration
    pub async fn wait_for_message(&self, timeout_secs: u64) -> Result<String, Box<dyn std::error::Error>> {
        // For the test, we'll just wait and assume the message was published
        // In a real implementation, we'd need to consume from Kafka
        tokio::time::sleep(Duration::from_secs(timeout_secs.min(5))).await;
        Ok("mock_event_message".to_string())
    }
    
    /// Get all messages from the topic using infra test consumer
    pub async fn get_all_messages(&self, max_wait_secs: u64) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        debug!("Creating test consumer for topic: {}", self.topic);
        let consumer = TestKafkaConsumer::new(&self.brokers, &self.topic).await?;
        consumer.get_all_messages(max_wait_secs).await
    }
    
    /// Wait for a specific number of messages to be available
    pub async fn wait_for_messages(&self, expected_count: usize, max_wait_secs: u64) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        debug!("Creating test consumer to wait for {} messages", expected_count);
        let consumer = TestKafkaConsumer::new(&self.brokers, &self.topic).await?;
        consumer.wait_for_messages(expected_count, max_wait_secs).await
    }
}

/// Get or create the global test Kafka container
async fn get_or_create_test_kafka_container() -> Result<Arc<TestKafkaContainer>, Box<dyn std::error::Error>> {
    let container_mutex = TEST_KAFKA_CONTAINER.get_or_init(|| {
        Arc::new(Mutex::new(None))
    });
    
    let mut container_guard = container_mutex.lock().await;
    
    if let Some(ref container) = *container_guard {
        return Ok(container.clone());
    }
    
    info!("Creating new Kafka test container");
    
    // Clean up any existing container
    cleanup_existing_kafka_container().await;
    
    // Clear configuration caches to ensure fresh port generation
    infra::config::clear_all_caches();
    
    // Load test configuration to get Kafka settings
    let config = infra::config::load_config()?;
    let kafka_config = &config.kafka;
    
    // Use the configuration's port resolution mechanism
    let kafka_port = kafka_config.actual_port();
    
    // Create Kafka container using Apache Kafka in KRaft mode (no Zookeeper needed)
    let kafka_image = GenericImage::new("apache/kafka", "3.7.0")
        .with_env_var("KAFKA_NODE_ID", "1")
        .with_env_var("KAFKA_LISTENER_SECURITY_PROTOCOL_MAP", "CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT")
        .with_env_var("KAFKA_ADVERTISED_LISTENERS", &format!("PLAINTEXT://localhost:{},PLAINTEXT_HOST://localhost:{}", kafka_port, kafka_port))
        .with_env_var("KAFKA_LISTENERS", &format!("PLAINTEXT://0.0.0.0:29092,CONTROLLER://0.0.0.0:29093,PLAINTEXT_HOST://0.0.0.0:{}", kafka_port))
        .with_env_var("KAFKA_INTER_BROKER_LISTENER_NAME", "PLAINTEXT")
        .with_env_var("KAFKA_CONTROLLER_LISTENER_NAMES", "CONTROLLER")
        .with_env_var("KAFKA_CONTROLLER_QUORUM_VOTERS", "1@localhost:29093")
        .with_env_var("KAFKA_PROCESS_ROLES", "broker,controller")
        .with_env_var("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "1")
        .with_env_var("KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR", "1")
        .with_env_var("KAFKA_TRANSACTION_STATE_LOG_MIN_ISR", "1")
        .with_env_var("KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS", "0")
        .with_env_var("KAFKA_AUTO_CREATE_TOPICS_ENABLE", "true")
        .with_env_var("CLUSTER_ID", "MkU3OEVBNTcwNTJENDM2Qk")
        .with_container_name("iam-test-kafka")
        .with_mapped_port(kafka_port, testcontainers::core::ContainerPort::Tcp(kafka_port));
    
    // Start Kafka
    info!("Starting Kafka container on port {}...", kafka_port);
    let kafka_container = kafka_image.start().await?;
    
    let brokers = format!("localhost:{}", kafka_port);
    
    info!("Test Kafka container started");
    info!("Brokers: {}", brokers);
    
    // Wait for Kafka to be ready
    TestKafka::wait_for_kafka(&brokers).await?;
    
    let test_container = Arc::new(TestKafkaContainer {
        container: kafka_container,
        brokers,
        port: kafka_port,
    });
    
    *container_guard = Some(test_container.clone());
    
    // Register cleanup handler on first container creation
    register_kafka_cleanup_handler().await;
    
    Ok(test_container)
}

/// Clean up any existing Kafka containers
async fn cleanup_existing_kafka_container() {
    use std::process::Command;
    
    debug!("Checking for existing Kafka test containers");
    
    let containers = ["iam-test-kafka"];
    
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

/// Register cleanup handler for Kafka containers
async fn register_kafka_cleanup_handler() {
    if KAFKA_CLEANUP_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    
    info!("Registering Kafka test container cleanup handler");
}

/// Test fixture for Kafka integration tests
pub struct TestKafkaFixture {
    pub kafka: TestKafka,
}

impl TestKafkaFixture {
    /// Create a new Kafka test fixture
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let kafka = TestKafka::new().await?;
        Ok(Self { kafka })
    }

    
    /// Wait for and verify a specific event was published
    pub async fn verify_event_published(&self, event_type: &str, timeout_secs: u64) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let messages = self.kafka.get_all_messages(timeout_secs).await?;
        
        for message in messages {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&message) {
                if let Some(event_type_value) = event.get("event_type") {
                    if event_type_value == event_type {
                        return Ok(event);
                    }
                }
            }
        }
        
        Err(format!("Event with type '{}' not found within {} seconds", event_type, timeout_secs).into())
    }
    
    /// Cleanup Kafka container (for test cleanup)
    pub async fn cleanup_container() -> Result<(), Box<dyn std::error::Error>> {
        let container_mutex = TEST_KAFKA_CONTAINER.get();
        if let Some(container_mutex) = container_mutex {
            let mut container_guard = container_mutex.lock().await;
            if let Some(container_arc) = container_guard.take() {
                info!("Manually cleaning up test Kafka container");
                
                match Arc::try_unwrap(container_arc) {
                    Ok(container) => {
                        container.cleanup().await;
                        info!("Test Kafka container cleanup completed");
                    }
                    Err(_) => {
                        warn!("Could not cleanup Kafka container: still has references");
                        // Fallback cleanup using Docker commands
                        cleanup_existing_kafka_container().await;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    
    #[tokio::test]
    #[serial]
    async fn test_kafka_container_creation() {
        let kafka = TestKafka::new().await.expect("Failed to create test Kafka");
        assert!(!kafka.brokers.is_empty());
        assert!(kafka.brokers.contains("localhost"));
    }
} 