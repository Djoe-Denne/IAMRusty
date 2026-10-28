//! SMTP `MailHog` test container for Telegraph email testing
//!
//! This module provides a `MailHog` SMTP container specifically for Telegraph integration tests
//! to verify email sending functionality.

use reqwest::Client;
use rustycog::config::load_config_part;
use rustycog::testing::testcontainers::{self, runners::AsyncRunner, GenericImage, ImageExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use telegraph_configuration::SmtpConfig;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Global test SMTP container instance
static TEST_SMTP_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestSmtpContainer>>>>> = OnceLock::new();

// Same Arc lease rule as the shared fixtures. Keep the slot intact when a
// consumer is active; an empty slot provides no identity to clean up.
fn take_unshared_smtp<T>(slot: &mut Option<Arc<T>>) -> Result<Option<T>, Arc<T>> {
    let Some(owned) = slot.take() else {
        return Ok(None);
    };
    match Arc::try_unwrap(owned) {
        Ok(owned) => Ok(Some(owned)),
        Err(shared) => {
            *slot = Some(shared.clone());
            Err(shared)
        }
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::take_unshared_smtp;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Debug)]
    struct Owner(Arc<AtomicUsize>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn smtp_unknown_and_active_consumers_are_preserved_without_cleanup() {
        let mut unknown: Option<Arc<Owner>> = None;
        assert!(take_unshared_smtp(&mut unknown).unwrap().is_none());
        let destroyed = Arc::new(AtomicUsize::new(0));
        let mut slot = Some(Arc::new(Owner(destroyed.clone())));
        let consumer = slot.as_ref().unwrap().clone();
        let active = take_unshared_smtp(&mut slot).unwrap_err();
        assert!(Arc::ptr_eq(slot.as_ref().unwrap(), &consumer));
        assert_eq!(destroyed.load(Ordering::SeqCst), 0);
        drop(active);
        drop(consumer);
        let owned = take_unshared_smtp(&mut slot).unwrap().unwrap();
        assert!(slot.is_none());
        assert_eq!(destroyed.load(Ordering::SeqCst), 0);
        drop(owned);
        assert_eq!(destroyed.load(Ordering::SeqCst), 1);
    }
}

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

/// `MailHog` SMTP test container
pub struct TestSmtpContainer {
    container: rustycog::testing::common::OwnedTestContainer,
}

impl TestSmtpContainer {
    /// Stop and remove the container
    pub async fn cleanup(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping the owned SMTP container by its tracked ID");
        self.container.stop().await?;
        self.container.rm().await?;
        Ok(())
    }
}

/// `MailHog` SMTP test container
pub struct TestSmtp {
    // Keep the existing Arc ownership lease alive for every fixture consumer.
    _container: Arc<TestSmtpContainer>,
    pub smtp_port: u16,
    pub api_port: u16,
    pub host: String,
    pub client: Client,
}

impl TestSmtp {
    /// Create a new SMTP test container with `MailHog`.
    ///
    /// # Errors
    ///
    /// Returns an error if the container cannot start, config cannot load, or `MailHog`
    /// does not become ready.
    pub async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        info!("Creating new MailHog SMTP test container");

        let smtp_config = load_config_part::<SmtpConfig>("communication.email.smtp")?;
        let owned = Self::get_or_create_smtp_container(&smtp_config).await?;

        // Use the configured ports directly since we mapped them
        let smtp_port = smtp_config.port;
        let api_port = 8025_u16;
        let host = "127.0.0.1".to_string();

        let client = Client::new();

        let container = Arc::new(Self {
            _container: owned,
            smtp_port,
            api_port,
            host,
            client,
        });

        // Wait for MailHog to be ready
        container.wait_for_ready().await?;
        container._container.container.ready()?;

        info!(
            "MailHog SMTP test container ready - SMTP: {}:{}, API: {}:{}",
            container.host, container.smtp_port, container.host, container.api_port
        );

        Ok(container)
    }

    async fn get_or_create_smtp_container(
        smtp_config: &SmtpConfig,
    ) -> Result<Arc<TestSmtpContainer>, Box<dyn std::error::Error>> {
        Self::cleanup_container().await?;

        // A prior test process may retain a diagnostic Exited(nonzero) fixture.
        // Do not adopt/remove it by name: isolate this new, independently owned
        // creation using the existing supervised named-fixture API.
        let container_name = format!("telegraph_test-smtp-{}", uuid::Uuid::new_v4().simple());
        let image = GenericImage::new("mailhog/mailhog", "latest")
            .with_container_name(&container_name)
            .with_mapped_port(
                smtp_config.port,
                testcontainers::core::ContainerPort::Tcp(1025),
            )
            .with_mapped_port(8025, testcontainers::core::ContainerPort::Tcp(8025));

        let owned = {
            let container_mutex = TEST_SMTP_CONTAINER.get_or_init(|| Arc::new(Mutex::new(None)));
            let mut container_guard = container_mutex.lock().await;
            let container = rustycog::testing::common::start_service_testcontainer(
                image.start(),
                "smtp",
                &container_name,
                smtp_config.port,
            )
            .await?;
            let owned = Arc::new(TestSmtpContainer { container });
            *container_guard = Some(owned.clone());
            owned
        };

        Ok(owned)
    }

    /// Cleanup SMTP container (for test cleanup).
    ///
    /// # Errors
    ///
    /// Returns an error for active fixture consumers or unverified owned teardown.
    /// Unknown containers are never discovered, adopted or removed by name.
    pub async fn cleanup_container() -> Result<(), Box<dyn std::error::Error>> {
        let container_mutex = TEST_SMTP_CONTAINER.get();
        if let Some(container_mutex) = container_mutex {
            let mut container_guard = container_mutex.lock().await;
            match take_unshared_smtp(&mut container_guard) {
                Ok(Some(container)) => container.cleanup().await?,
                Ok(None) => {}
                Err(container) => {
                    container.container.deferred();
                    return Err("SMTP cleanup deferred: active fixture consumers".into());
                }
            }
        }
        rustycog::testing::common::join_fixture_creations().await?;
        Ok(())
    }

    /// Wait for `MailHog` to be ready
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

    /// Get all emails sent to `MailHog`.
    ///
    /// # Errors
    ///
    /// Returns an error if the `MailHog` API request or JSON parse fails.
    pub async fn get_emails(&self) -> Result<Vec<TestEmail>, Box<dyn std::error::Error>> {
        let api_url = format!("http://{}:{}/api/v1/messages", self.host, self.api_port);

        let response = self.client.get(&api_url).send().await?;
        let text = response.text().await?;

        // MailHog returns emails in a specific format, parse them
        let emails: serde_json::Value = serde_json::from_str(&text)?;

        let mut test_emails = Vec::new();

        if let Some(items) = emails.as_array() {
            for item in items {
                if let Ok(email) = parse_mailhog_message(item) {
                    test_emails.push(email);
                }
            }
        }

        Ok(test_emails)
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

    /// Clear all emails from `MailHog`.
    ///
    /// # Errors
    ///
    /// Returns an error if the `MailHog` delete request fails.
    pub async fn clear_emails(&self) -> Result<(), Box<dyn std::error::Error>> {
        let api_url = format!("http://{}:{}/api/v1/messages", self.host, self.api_port);
        self.client.delete(&api_url).send().await?;
        Ok(())
    }

    /// Get SMTP configuration for Telegraph
    #[must_use]
    pub fn smtp_host(&self) -> &str {
        &self.host
    }

    #[must_use]
    pub const fn smtp_port(&self) -> u16 {
        self.smtp_port
    }
}

fn parse_mailhog_message(
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
        name: String::new(),
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

fn parse_mailhog_recipients(message: &serde_json::Value) -> Vec<EmailAddress> {
    message
        .get("To")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_object())
        .map(|obj| EmailAddress {
            name: String::new(),
            address: mailhog_address(obj),
        })
        .collect()
}

fn mailhog_address(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let mailbox = obj.get("Mailbox").and_then(|v| v.as_str()).unwrap_or("");
    let domain = obj.get("Domain").and_then(|v| v.as_str()).unwrap_or("");

    if mailbox.is_empty() || domain.is_empty() {
        String::new()
    } else {
        format!("{mailbox}@{domain}")
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
