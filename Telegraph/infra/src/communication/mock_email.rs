//! Mock email service for testing

use async_trait::async_trait;
use telegraph_domain::{DomainError, EmailService};
use tracing::{info, debug};
use std::sync::{Arc, Mutex};

/// Mock email adapter that captures sent emails for testing
#[derive(Debug, Clone)]
pub struct MockEmailAdapter {
    sent_emails: Arc<Mutex<Vec<SentEmail>>>,
}

/// Represents an email that was "sent" during testing
#[derive(Debug, Clone)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
    pub attachment_count: usize,
}

impl MockEmailAdapter {
    /// Create a new mock email adapter
    pub fn new() -> Self {
        Self {
            sent_emails: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Get all emails that were "sent" during testing
    pub fn sent_emails(&self) -> Vec<SentEmail> {
        self.sent_emails.lock().unwrap().clone()
    }
    
    /// Clear the list of sent emails
    pub fn clear_sent_emails(&self) {
        self.sent_emails.lock().unwrap().clear();
    }
    
    /// Check if an email was sent to a specific recipient
    pub fn was_email_sent_to(&self, recipient: &str) -> bool {
        self.sent_emails
            .lock()
            .unwrap()
            .iter()
            .any(|email| email.to == recipient)
    }
    
    /// Get emails sent to a specific recipient
    pub fn emails_for_recipient(&self, recipient: &str) -> Vec<SentEmail> {
        self.sent_emails
            .lock()
            .unwrap()
            .iter()
            .filter(|email| email.to == recipient)
            .cloned()
            .collect()
    }
}

impl Default for MockEmailAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailService for MockEmailAdapter {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        text_body: &str,
        html_body: Option<&str>,
        attachments: &[telegraph_domain::port::communication::EmailAttachment],
    ) -> Result<String, DomainError> {
        info!(
            to = to,
            subject = subject,
            has_html = html_body.is_some(),
            attachments = attachments.len(),
            "Mock email service: simulating email send"
        );
        
        let sent_email = SentEmail {
            to: to.to_string(),
            subject: subject.to_string(),
            text_body: text_body.to_string(),
            html_body: html_body.map(|s| s.to_string()),
            attachment_count: attachments.len(),
        };
        
        self.sent_emails.lock().unwrap().push(sent_email);
        
        // Return a mock delivery ID
        let delivery_id = format!("mock-delivery-{}", uuid::Uuid::new_v4());
        
        info!(
            delivery_id = %delivery_id,
            to = to,
            "Mock email service: email 'sent' successfully"
        );
        
        Ok(delivery_id)
    }
    
    fn validate_email(&self, email: &str) -> Result<(), DomainError> {
        if email.contains('@') && email.len() > 3 {
            Ok(())
        } else {
            Err(DomainError::invalid_email(email.to_string()))
        }
    }
    
    async fn health_check(&self) -> Result<(), DomainError> {
        debug!("Mock email service: health check always passes");
        Ok(())
    }
} 