//! Email communication adapter

use async_trait::async_trait;
use telegraph_domain::{DomainError, port::communication::EmailProvider, entity::communication::EmailCommunication};
use lettre::{AsyncTransport, AsyncSmtpTransport, Tokio1Executor, Message, message::header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use tracing::{info, error, debug};

/// Email adapter configuration
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: "Telegraph".to_string(),
            use_tls: true,
        }
    }
}

/// Email adapter using SMTP
pub struct EmailAdapter {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    config: EmailConfig,
}

impl EmailAdapter {
    /// Create a new email adapter with configuration
    pub fn new(config: EmailConfig) -> Result<Self, DomainError> {
        let mailer = Self::create_mailer(&config)?;
        
        Ok(Self {
            mailer,
            config,
        })
    }
    
    /// Create a new email adapter with default configuration (for testing)
    pub fn new_default() -> Self {
        let config = EmailConfig::default();
        Self {
            mailer: Self::create_test_mailer(),
            config,
        }
    }
    
    /// Create SMTP mailer from configuration
    fn create_mailer(config: &EmailConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, DomainError> {
        let mut mailer_builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| DomainError::InfrastructureError(format!("Failed to create SMTP relay: {}", e)))?;
        
        // Set port
        mailer_builder = mailer_builder.port(config.smtp_port);
        
        // Set authentication if credentials are provided
        if !config.smtp_username.is_empty() {
            let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
            mailer_builder = mailer_builder.credentials(creds);
        }
        
        // Set TLS configuration
        if config.use_tls {
            let tls_params = TlsParameters::new(config.smtp_host.clone())
                .map_err(|e| DomainError::InfrastructureError(format!("Failed to create TLS parameters: {}", e)))?;
            mailer_builder = mailer_builder.tls(Tls::Required(tls_params));
        } else {
            mailer_builder = mailer_builder.tls(Tls::None);
        }
        
        Ok(mailer_builder.build())
    }
    
    /// Create a test mailer that doesn't actually send emails
    fn create_test_mailer() -> AsyncSmtpTransport<Tokio1Executor> {
        // Use a simple builder for testing that won't actually send emails
        AsyncSmtpTransport::<Tokio1Executor>::unencrypted_localhost()
    }
}

#[async_trait]
impl EmailProvider for EmailAdapter {
    async fn send_email(
        &self,
        email: &EmailCommunication,
    ) -> Result<String, DomainError> {
        info!(
            to = email.recipient.email.as_ref().unwrap(),
            subject = email.subject,
            has_html = email.html_body.is_some(),
            "Sending email via SMTP"
        );
        
        // Build the email message
        let from_address = format!("{} <{}>", self.config.from_name, self.config.from_email);
        let mut message_builder = Message::builder()
            .from(from_address.parse().map_err(|e| DomainError::InfrastructureError(format!("Invalid from address: {}", e)))?)
            .to(email.recipient.email.as_ref().unwrap().parse().map_err(|e| DomainError::invalid_email(format!("Invalid to address: {}", e)))?)
            .subject(email.subject.clone());

        // Set message body
        let message = if let Some(html) = email.html_body.clone() {
            // Send both text and HTML
            let body = lettre::message::MultiPart::alternative()
                .singlepart(lettre::message::SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(email.text_body.to_string()))
                .singlepart(lettre::message::SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html.to_string()));
            
            message_builder.multipart(body)
        } else {
            // Send only text
            message_builder.body(email.text_body.to_string())
        }.map_err(|e| DomainError::InfrastructureError(format!("Failed to build email message: {}", e)))?;

        // Send the email
        match self.mailer.send(message).await {
            Ok(response) => {
                let delivery_id = response.first_line().unwrap_or("unknown").to_string();
                info!(
                    delivery_id = %delivery_id,
                    to = email.recipient.email.as_ref().unwrap(),
                    "Email sent successfully via SMTP"
                );
                Ok(delivery_id)
            }
            Err(e) => {
                error!(
                    error = %e,
                    to = email.recipient.email.as_ref().unwrap(),
                    subject = email.subject,
                    "Failed to send email via SMTP"
                );
                Err(DomainError::InfrastructureError(format!("Failed to send email: {}", e)))
            }
        }
    }
    
    fn validate_email(&self, email: &str) -> Result<(), DomainError> {
        if email.contains('@') && email.len() > 3 {
            Ok(())
        } else {
            Err(DomainError::invalid_email(email.to_string()))
        }
    }
    
    async fn health_check(&self) -> Result<(), DomainError> {
        debug!("Performing email service health check");
        
        // Test SMTP connection by checking if we can connect
        match self.mailer.test_connection().await {
            Ok(is_connected) => {
                if is_connected {
                    debug!("✅ Email service health check passed - SMTP connection successful");
                    Ok(())
                } else {
                    error!("❌ Email service health check failed - SMTP connection failed");
                    Err(DomainError::InfrastructureError("SMTP connection test failed".to_string()))
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    "❌ Email service health check failed - SMTP connection error"
                );
                Err(DomainError::InfrastructureError(format!("SMTP health check error: {}", e)))
            }
        }
    }
} 