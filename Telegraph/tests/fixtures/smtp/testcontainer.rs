//! SMTP MailHog test container for Telegraph email testing
//!
//! This module provides a MailHog SMTP container specifically for Telegraph integration tests
//! to verify email sending functionality.

use reqwest::Client;
use rustycog_config::load_config_part;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use telegraph_configuration::SmtpConfig;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Global test SMTP container instance
static TEST_SMTP_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestSmtpContainer>>>>> = OnceLock::new();

/// Test email structure for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEmail {
    pub id: String,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub subject: String,
    pub text: String,
    pub html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub name: String,
    pub address: String,
}

/// MailHog SMTP test container
pub struct TestSmtpContainer {
    container: ContainerAsync<GenericImage>,
}

impl TestSmtpContainer {
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

/// MailHog SMTP test container
pub struct TestSmtp {
    pub smtp_port: u16,
    pub api_port: u16,
    pub host: String,
    pub client: Client,
}

impl TestSmtp {
    /// Create a new SMTP test container with MailHog
    pub async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        info!("Creating new MailHog SMTP test container");

        let smtp_config = load_config_part::<SmtpConfig>("communication.email.smtp")?;
        let _ = Self::get_or_create_smtp_container(&smtp_config).await?;

        // Use the configured ports directly since we mapped them
        let smtp_port = smtp_config.port as u16;
        let api_port = 8025 as u16;
        let host = "127.0.0.1".to_string();

        let client = Client::new();

        let container = Arc::new(Self {
            smtp_port,
            api_port,
            host,
            client,
        });

        // Wait for MailHog to be ready
        container.wait_for_ready().await?;

        info!(
            "MailHog SMTP test container ready - SMTP: {}:{}, API: {}:{}",
            container.host, container.smtp_port, container.host, container.api_port
        );

        Ok(container)
    }

    async fn get_or_create_smtp_container(
        smtp_config: &SmtpConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::cleanup_container().await?;
        let container_mutex = TEST_SMTP_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));

        let mut container_guard = container_mutex.lock().await;

        // MailHog image with mapped ports
        let image = GenericImage::new("mailhog/mailhog", "latest")
            .with_container_name("telegraph_test-smtp")
            .with_mapped_port(
                smtp_config.port,
                testcontainers::core::ContainerPort::Tcp(1025),
            ) // Map test.toml port to MailHog SMTP
            .with_mapped_port(8025, testcontainers::core::ContainerPort::Tcp(8025)); // TODO: fetch it from config

        let container = image.start().await?;

        *container_guard = Some(Arc::new(TestSmtpContainer { container }));

        Ok(())
    }

    /// Cleanup SQS container (for test cleanup)
    pub async fn cleanup_container() -> Result<(), Box<dyn std::error::Error>> {
        let container_mutex = TEST_SMTP_CONTAINER.get();
        if let Some(container_mutex) = container_mutex {
            let mut container_guard = container_mutex.lock().await;
            if let Some(container_arc) = container_guard.take() {
                info!("Manually cleaning up test SMTP container");

                match Arc::try_unwrap(container_arc) {
                    Ok(container) => {
                        container.cleanup().await;
                        info!("Test SMTP container cleanup completed");
                    }
                    Err(_) => {
                        warn!("Could not cleanup SMTP container: still has references");
                    }
                }
            }
        }

        // Fallback cleanup using Docker commands
        Self::cleanup_existing_smtp_container().await;
        Ok(())
    }

    /// Clean up any existing SMTP containers
    async fn cleanup_existing_smtp_container() {
        use std::process::Command;

        debug!("Checking for existing SMTP LocalStack test containers");

        let containers = ["telegraph_test-smtp"];

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

    /// Wait for MailHog to be ready
    async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let api_url = format!("http://{}:{}/api/v1/messages", self.host, self.api_port);

        for _ in 0..30 {
            match self.client.get(&api_url).send().await {
                Ok(response) if response.status().is_success() => {
                    debug!("MailHog is ready");
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        Err("MailHog failed to become ready within timeout".into())
    }

    /// Get all emails sent to MailHog
    pub async fn get_emails(&self) -> Result<Vec<TestEmail>, Box<dyn std::error::Error>> {
        let api_url = format!("http://{}:{}/api/v1/messages", self.host, self.api_port);

        let response = self.client.get(&api_url).send().await?;
        let text = response.text().await?;

        // MailHog returns emails in a specific format, parse them
        let emails: serde_json::Value = serde_json::from_str(&text)?;

        let mut test_emails = Vec::new();

        if let Some(items) = emails.as_array() {
            for item in items {
                if let Ok(email) = self.parse_mailhog_message(item) {
                    test_emails.push(email);
                }
            }
        }

        Ok(test_emails)
    }

    /// Parse a MailHog message into TestEmail
    fn parse_mailhog_message(
        &self,
        message: &serde_json::Value,
    ) -> Result<TestEmail, Box<dyn std::error::Error>> {
        let id = message
            .get("ID")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let from_obj = message
            .get("From")
            .and_then(|v| v.as_object())
            .ok_or("Missing From field")?;

        let to_addresses = parse_mailhog_recipients(message);
        let content = message
            .get("Content")
            .and_then(|v| v.as_object())
            .ok_or("Missing Content field")?;
        let headers = content
            .get("Headers")
            .and_then(|v| v.as_object())
            .ok_or("Missing Headers field")?;
        let subject = mailhog_header(headers, "Subject");
        let (text_content, html_content) = parse_mailhog_bodies(message);

        let from = EmailAddress {
            name: "".to_string(),
            address: mailhog_address(from_obj),
        };

        Ok(TestEmail {
            id,
            from,
            to: to_addresses,
            subject,
            text: text_content,
            html: html_content,
        })
    }

    /// Get count of emails sent
    pub async fn email_count(&self) -> usize {
        self.get_emails().await.unwrap_or_default().len()
    }

    /// Check if an email with specific subject and recipient was sent
    pub async fn has_email(&self, subject: &str, recipient: &str) -> bool {
        info!(
            "🔍 Checking for email - Subject: '{}', Recipient: '{}', API Port: {}",
            subject, recipient, self.api_port
        );
        let emails = self.get_emails().await.unwrap_or_default();
        info!("📧 Retrieved {} emails from MailHog", emails.len());
        for (i, email) in emails.iter().enumerate() {
            info!(
                "📧 Email {}: Subject='{}', To={:?}",
                i, email.subject, email.to
            );
        }

        let found = emails.iter().any(|email| {
            email.subject.contains(subject)
                && email.to.iter().any(|addr| addr.address.contains(recipient))
        });

        info!("📧 Email found: {}", found);
        found
    }

    /// Clear all emails from MailHog
    pub async fn clear_emails(&self) -> Result<(), Box<dyn std::error::Error>> {
        let api_url = format!("http://{}:{}/api/v1/messages", self.host, self.api_port);
        self.client.delete(&api_url).send().await?;
        Ok(())
    }

    /// Get SMTP configuration for Telegraph
    pub fn smtp_host(&self) -> &str {
        &self.host
    }

    pub fn smtp_port(&self) -> u16 {
        self.smtp_port
    }
}

fn parse_mailhog_recipients(message: &serde_json::Value) -> Vec<EmailAddress> {
    message
        .get("To")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_object())
        .map(|obj| EmailAddress {
            name: "".to_string(),
            address: mailhog_address(obj),
        })
        .collect()
}

fn mailhog_address(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let mailbox = obj.get("Mailbox").and_then(|v| v.as_str()).unwrap_or("");
    let domain = obj.get("Domain").and_then(|v| v.as_str()).unwrap_or("");

    if mailbox.is_empty() || domain.is_empty() {
        "".to_string()
    } else {
        format!("{}@{}", mailbox, domain)
    }
}

fn mailhog_header(headers: &serde_json::Map<String, serde_json::Value>, name: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn parse_mailhog_bodies(message: &serde_json::Value) -> (String, String) {
    let mut text_content = String::new();
    let mut html_content = String::new();

    for part in mailhog_mime_parts(message) {
        let content_type = mailhog_part_content_type(part);
        let body = part.get("Body").and_then(|v| v.as_str()).unwrap_or("");

        if content_type.contains("text/plain") {
            text_content = body.to_string();
        } else if content_type.contains("text/html") {
            html_content = body.to_string();
        }
    }

    (text_content, html_content)
}

fn mailhog_mime_parts(message: &serde_json::Value) -> Vec<&serde_json::Value> {
    message
        .get("MIME")
        .and_then(|v| v.get("Parts"))
        .and_then(|v| v.as_array())
        .map(|parts| parts.iter().collect())
        .unwrap_or_default()
}

fn mailhog_part_content_type(part: &serde_json::Value) -> &str {
    part.get("Headers")
        .and_then(|v| v.get("Content-Type"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
}
