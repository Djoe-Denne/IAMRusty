use super::resources::*;
use rustycog_testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{body_string_contains, method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

/// SMTP service for mocking SMTP server endpoints
pub struct SmtpService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture, // Keeps the fixture alive for automatic cleanup
}

impl SmtpService {
    /// Create a new SMTP service instance with automatic mock cleanup
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();

        Self {
            server,
            _fixture: fixture,
        }
    }

    /// Get the base URL for SMTP mocking (used as SMTP host)
    pub fn host(&self) -> String {
        // Extract just the host part from the URI
        let uri = self.server.uri();
        uri.replace("http://", "").replace("https://", "")
    }

    /// Get the port for SMTP mocking
    pub fn port(&self) -> u16 {
        self.server.address().port()
    }

    /// Get the full server URI
    pub fn uri(&self) -> String {
        self.server.uri()
    }

    /// Manual reset of all mocks (also happens automatically when service is dropped)
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    /// Mock SMTP connection handshake (220 response)
    /// This mocks the initial SMTP greeting
    pub async fn mock_greeting(&self, response: SmtpResponse) -> &Self {
        Mock::given(method("POST"))
            .and(path("/smtp/connect"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock EHLO command response
    pub async fn mock_ehlo(&self, capabilities: SmtpCapabilities) -> &Self {
        Mock::given(method("POST"))
            .and(path("/smtp/ehlo"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&capabilities)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock SMTP authentication
    pub async fn mock_auth(&self, auth_request: SmtpAuthRequest, response: SmtpResponse) -> &Self {
        Mock::given(method("POST"))
            .and(path("/smtp/auth"))
            .and(body_string_contains(&auth_request.username))
            .and(body_string_contains(&auth_request.auth_method))
            .respond_with(
                ResponseTemplate::new(if response.code >= 400 { 400 } else { 200 })
                    .set_body_json(&response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock MAIL FROM command
    pub async fn mock_mail_from(&self, from_email: &str, response: SmtpResponse) -> &Self {
        Mock::given(method("POST"))
            .and(path("/smtp/mail_from"))
            .and(body_string_contains(from_email))
            .respond_with(
                ResponseTemplate::new(if response.code >= 400 { 400 } else { 200 })
                    .set_body_json(&response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock RCPT TO command
    pub async fn mock_rcpt_to(&self, to_email: &str, response: SmtpResponse) -> &Self {
        Mock::given(method("POST"))
            .and(path("/smtp/rcpt_to"))
            .and(body_string_contains(to_email))
            .respond_with(
                ResponseTemplate::new(if response.code >= 400 { 400 } else { 200 })
                    .set_body_json(&response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock DATA command (email content)
    pub async fn mock_data(&self, expected_email: &SmtpEmail, response: SmtpResponse) -> &Self {
        let mut mock = Mock::given(method("POST")).and(path("/smtp/data"));

        // Add matchers for email content
        mock = mock.and(body_string_contains(&expected_email.subject));

        for to_addr in &expected_email.to {
            mock = mock.and(body_string_contains(to_addr));
        }

        if let Some(text_body) = &expected_email.text_body {
            // Match part of the text body to allow for template variations
            let text_words: Vec<&str> = text_body.split_whitespace().take(3).collect();
            for word in text_words {
                if word.len() > 2 {
                    // Skip short words like "Hi"
                    mock = mock.and(body_string_contains(word));
                }
            }
        }

        mock.respond_with(
            ResponseTemplate::new(if response.code >= 400 { 400 } else { 200 })
                .set_body_json(&response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&self.server)
        .await;

        self
    }

    /// Mock QUIT command
    pub async fn mock_quit(&self, response: SmtpResponse) -> &Self {
        Mock::given(method("POST"))
            .and(path("/smtp/quit"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock full successful email sending sequence
    pub async fn mock_successful_email_send(&self, expected_email: &SmtpEmail) -> &Self {
        // Mock all SMTP commands for a successful email send
        self.mock_greeting(SmtpResponse::service_ready())
            .await
            .mock_ehlo(SmtpCapabilities::default_localhost())
            .await
            .mock_mail_from(&expected_email.from, SmtpResponse::ok())
            .await;

        // Mock RCPT TO for each recipient
        for to_addr in &expected_email.to {
            self.mock_rcpt_to(to_addr, SmtpResponse::ok()).await;
        }

        self.mock_data(
            expected_email,
            SmtpResponse::message_accepted(
                &expected_email
                    .message_id
                    .clone()
                    .unwrap_or("test-msg-123".to_string()),
            ),
        )
        .await
        .mock_quit(SmtpResponse::closing())
        .await
    }

    /// Mock email sending with authentication
    pub async fn mock_authenticated_email_send(
        &self,
        auth_request: SmtpAuthRequest,
        expected_email: &SmtpEmail,
    ) -> &Self {
        self.mock_greeting(SmtpResponse::service_ready())
            .await
            .mock_ehlo(SmtpCapabilities::default_localhost())
            .await
            .mock_auth(auth_request, SmtpResponse::auth_success())
            .await
            .mock_mail_from(&expected_email.from, SmtpResponse::ok())
            .await;

        // Mock RCPT TO for each recipient
        for to_addr in &expected_email.to {
            self.mock_rcpt_to(to_addr, SmtpResponse::ok()).await;
        }

        self.mock_data(
            expected_email,
            SmtpResponse::message_accepted(
                &expected_email
                    .message_id
                    .clone()
                    .unwrap_or("test-msg-123".to_string()),
            ),
        )
        .await
        .mock_quit(SmtpResponse::closing())
        .await
    }

    /// Mock authentication failure
    pub async fn mock_auth_failure(&self, auth_request: SmtpAuthRequest) -> &Self {
        self.mock_greeting(SmtpResponse::service_ready())
            .await
            .mock_ehlo(SmtpCapabilities::default_localhost())
            .await
            .mock_auth(auth_request, SmtpResponse::auth_failed())
            .await
    }

    /// Mock recipient rejection (550 error)
    pub async fn mock_recipient_rejection(&self, to_email: &str) -> &Self {
        self.mock_greeting(SmtpResponse::service_ready())
            .await
            .mock_ehlo(SmtpCapabilities::default_localhost())
            .await
            .mock_mail_from("noreply@telegraph.com", SmtpResponse::ok())
            .await
            .mock_rcpt_to(to_email, SmtpResponse::mailbox_unavailable())
            .await
    }

    /// Get all received requests for inspection
    pub async fn received_requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    /// Check if a specific email was sent by inspecting requests
    pub async fn verify_email_sent(
        &self,
        expected_subject: &str,
        expected_recipient: &str,
    ) -> bool {
        let requests = self.received_requests().await;

        // Look for DATA request containing our email
        requests.iter().any(|req| {
            if req.url.path() == "/smtp/data" {
                let body = String::from_utf8_lossy(&req.body);
                body.contains(expected_subject) && body.contains(expected_recipient)
            } else {
                false
            }
        })
    }

    /// Get count of emails sent (DATA commands received)
    pub async fn email_count(&self) -> usize {
        let requests = self.received_requests().await;
        requests
            .iter()
            .filter(|req| req.url.path() == "/smtp/data")
            .count()
    }

    /// Advanced mock for testing specific SMTP scenarios
    pub async fn mock_custom_scenario(&self) -> SmtpScenarioBuilder<'_> {
        SmtpScenarioBuilder::new(self)
    }
}

/// Builder for complex SMTP test scenarios
pub struct SmtpScenarioBuilder<'a> {
    service: &'a SmtpService,
}

impl<'a> SmtpScenarioBuilder<'a> {
    const fn new(service: &'a SmtpService) -> Self {
        Self { service }
    }

    /// Start with greeting
    pub async fn greeting(self, response: SmtpResponse) -> Self {
        self.service.mock_greeting(response).await;
        self
    }

    /// Add EHLO response
    pub async fn ehlo(self, capabilities: SmtpCapabilities) -> Self {
        self.service.mock_ehlo(capabilities).await;
        self
    }

    /// Add authentication step
    pub async fn auth(self, auth_request: SmtpAuthRequest, response: SmtpResponse) -> Self {
        self.service.mock_auth(auth_request, response).await;
        self
    }

    /// Add MAIL FROM step
    pub async fn mail_from(self, from: &str, response: SmtpResponse) -> Self {
        self.service.mock_mail_from(from, response).await;
        self
    }

    /// Add RCPT TO step
    pub async fn rcpt_to(self, to: &str, response: SmtpResponse) -> Self {
        self.service.mock_rcpt_to(to, response).await;
        self
    }

    /// Add DATA step
    pub async fn data(self, email: &SmtpEmail, response: SmtpResponse) -> Self {
        self.service.mock_data(email, response).await;
        self
    }

    /// Add QUIT step
    pub async fn quit(self, response: SmtpResponse) -> Self {
        self.service.mock_quit(response).await;
        self
    }

    /// Finish building the scenario
    pub async fn build(self) -> &'a SmtpService {
        self.service
    }
}
