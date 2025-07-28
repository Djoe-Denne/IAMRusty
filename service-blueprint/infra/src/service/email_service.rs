use async_trait::async_trait;

use {{SERVICE_NAME}}_domain::{DomainError, EmailService};

/// Dummy email service for testing and development
pub struct DummyEmailService;

impl DummyEmailService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EmailService for DummyEmailService {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        html_body: Option<&str>,
    ) -> Result<(), DomainError> {
        tracing::info!(
            to = to,
            subject = subject,
            body_length = body.len(),
            has_html = html_body.is_some(),
            "Dummy email sent"
        );
        Ok(())
    }

    async fn send_templated_email(
        &self,
        to: &str,
        template_id: &str,
        template_data: &serde_json::Value,
    ) -> Result<(), DomainError> {
        tracing::info!(
            to = to,
            template_id = template_id,
            template_data = %template_data,
            "Dummy templated email sent"
        );
        Ok(())
    }
}

/// SMTP email service implementation
#[cfg(feature = "lettre")]
pub struct SmtpEmailService {
    // SMTP configuration would go here
    from_address: String,
    from_name: String,
}

#[cfg(feature = "lettre")]
impl SmtpEmailService {
    pub fn new(from_address: String, from_name: String) -> Self {
        Self {
            from_address,
            from_name,
        }
    }
}

#[cfg(feature = "lettre")]
#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send_email(
        &self,
        _to: &str,
        _subject: &str,
        _body: &str,
        _html_body: Option<&str>,
    ) -> Result<(), DomainError> {
        // Implementation would use lettre SMTP client
        todo!("Implement SMTP email sending")
    }

    async fn send_templated_email(
        &self,
        _to: &str,
        _template_id: &str,
        _template_data: &serde_json::Value,
    ) -> Result<(), DomainError> {
        // Implementation would render template and send via SMTP
        todo!("Implement SMTP templated email sending")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dummy_email_service() {
        let service = DummyEmailService::new();

        let result = service
            .send_email(
                "test@example.com",
                "Test Subject",
                "Test Body",
                Some("<h1>Test HTML Body</h1>"),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dummy_templated_email_service() {
        let service = DummyEmailService::new();
        let template_data = serde_json::json!({
            "name": "John Doe",
            "action_url": "https://example.com/action"
        });

        let result = service
            .send_templated_email("test@example.com", "welcome", &template_data)
            .await;

        assert!(result.is_ok());
    }
} 