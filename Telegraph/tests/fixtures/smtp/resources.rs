use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// SMTP email message structure for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpEmail {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub message_id: Option<String>,
}

impl SmtpEmail {
    /// Create a builder for custom email data
    pub fn create() -> SmtpEmailBuilder {
        SmtpEmailBuilder::default()
    }

    /// Pre-built email: User signup welcome
    pub fn user_signup_welcome(user_email: &str, username: &str) -> Self {
        Self {
            from: "noreply@telegraph.com".to_string(),
            to: vec![user_email.to_string()],
            cc: None,
            bcc: None,
            subject: "Welcome to AI For All!".to_string(),
            text_body: Some(format!("Welcome {}, thank you for signing up!", username)),
            html_body: Some(format!(
                "<h1>Welcome {}!</h1><p>Thank you for signing up to AI For All.</p>", 
                username
            )),
            message_id: Some("test-message-id-123".to_string()),
        }
    }

    /// Pre-built email: Password reset request
    pub fn password_reset_request(user_email: &str, username: &str) -> Self {
        Self {
            from: "noreply@telegraph.com".to_string(),
            to: vec![user_email.to_string()],
            cc: None,
            bcc: None,
            subject: "Password Reset Request".to_string(),
            text_body: Some(format!("Hi {}, you requested a password reset.", username)),
            html_body: Some(format!(
                "<h1>Password Reset</h1><p>Hi {}, you requested a password reset.</p>", 
                username
            )),
            message_id: Some("test-password-reset-456".to_string()),
        }
    }

    /// Pre-built email: Email verification
    pub fn email_verification(user_email: &str, username: &str) -> Self {
        Self {
            from: "noreply@telegraph.com".to_string(),
            to: vec![user_email.to_string()],
            cc: None,
            bcc: None,
            subject: "Email Verification".to_string(),
            text_body: Some(format!("Hi {}, please verify your email address.", username)),
            html_body: Some(format!(
                "<h1>Email Verification</h1><p>Hi {}, please verify your email address.</p>", 
                username
            )),
            message_id: Some("test-email-verify-789".to_string()),
        }
    }
}

/// Builder for SMTP email data
#[derive(Debug, Default)]
pub struct SmtpEmailBuilder {
    from: Option<String>,
    to: Vec<String>,
    cc: Option<Vec<String>>,
    bcc: Option<Vec<String>>,
    subject: Option<String>,
    text_body: Option<String>,
    html_body: Option<String>,
    message_id: Option<String>,
}

impl SmtpEmailBuilder {
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to.push(to.into());
        self
    }

    pub fn cc(mut self, cc: Vec<String>) -> Self {
        self.cc = Some(cc);
        self
    }

    pub fn bcc(mut self, bcc: Vec<String>) -> Self {
        self.bcc = Some(bcc);
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }

    pub fn html_body(mut self, html_body: impl Into<String>) -> Self {
        self.html_body = Some(html_body.into());
        self
    }

    pub fn message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    pub fn build(self) -> SmtpEmail {
        SmtpEmail {
            from: self.from.unwrap_or_else(|| "noreply@telegraph.com".to_string()),
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            subject: self.subject.unwrap_or_else(|| "Test Subject".to_string()),
            text_body: self.text_body,
            html_body: self.html_body,
            message_id: self.message_id,
        }
    }
}

/// SMTP authentication request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpAuthRequest {
    pub username: String,
    pub password: String,
    pub auth_method: String, // PLAIN, LOGIN, CRAM-MD5, etc.
}

impl SmtpAuthRequest {
    pub fn plain(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            auth_method: "PLAIN".to_string(),
        }
    }

    pub fn login(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            auth_method: "LOGIN".to_string(),
        }
    }
}

/// SMTP connection request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConnectionRequest {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub use_starttls: bool,
}

impl SmtpConnectionRequest {
    pub fn localhost(port: u16) -> Self {
        Self {
            host: "localhost".to_string(),
            port,
            use_tls: false,
            use_starttls: false,
        }
    }

    pub fn with_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }

    pub fn with_starttls(mut self, use_starttls: bool) -> Self {
        self.use_starttls = use_starttls;
        self
    }
}

/// SMTP response codes and messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpResponse {
    pub code: u16,
    pub message: String,
    pub enhanced_status_code: Option<String>,
}

impl SmtpResponse {
    /// 220 Service ready
    pub fn service_ready() -> Self {
        Self {
            code: 220,
            message: "localhost ESMTP Postfix".to_string(),
            enhanced_status_code: None,
        }
    }

    /// 250 Requested mail action okay, completed
    pub fn ok() -> Self {
        Self {
            code: 250,
            message: "OK".to_string(),
            enhanced_status_code: Some("2.0.0".to_string()),
        }
    }

    /// 250 Authentication successful
    pub fn auth_success() -> Self {
        Self {
            code: 235,
            message: "Authentication successful".to_string(),
            enhanced_status_code: Some("2.7.0".to_string()),
        }
    }

    /// 354 Start mail input
    pub fn start_mail_input() -> Self {
        Self {
            code: 354,
            message: "End data with <CR><LF>.<CR><LF>".to_string(),
            enhanced_status_code: None,
        }
    }

    /// 250 Message accepted for delivery
    pub fn message_accepted(message_id: &str) -> Self {
        Self {
            code: 250,
            message: format!("Message accepted for delivery id={}", message_id),
            enhanced_status_code: Some("2.0.0".to_string()),
        }
    }

    /// 221 Service closing transmission channel
    pub fn closing() -> Self {
        Self {
            code: 221,
            message: "Bye".to_string(),
            enhanced_status_code: Some("2.0.0".to_string()),
        }
    }

    /// 535 Authentication failed
    pub fn auth_failed() -> Self {
        Self {
            code: 535,
            message: "Authentication failed".to_string(),
            enhanced_status_code: Some("5.7.8".to_string()),
        }
    }

    /// 550 Requested action not taken: mailbox unavailable
    pub fn mailbox_unavailable() -> Self {
        Self {
            code: 550,
            message: "Requested action not taken: mailbox unavailable".to_string(),
            enhanced_status_code: Some("5.1.1".to_string()),
        }
    }
}

/// SMTP server capabilities (EHLO response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpCapabilities {
    pub hostname: String,
    pub extensions: Vec<String>,
}

impl SmtpCapabilities {
    pub fn default_localhost() -> Self {
        Self {
            hostname: "localhost".to_string(),
            extensions: vec![
                "PIPELINING".to_string(),
                "SIZE 10240000".to_string(),
                "VRFY".to_string(),
                "ETRN".to_string(),
                "STARTTLS".to_string(),
                "AUTH PLAIN LOGIN CRAM-MD5".to_string(),
                "AUTH=PLAIN LOGIN CRAM-MD5".to_string(),
                "ENHANCEDSTATUSCODES".to_string(),
                "8BITMIME".to_string(),
                "DSN".to_string(),
            ],
        }
    }

    pub fn to_response_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("250-{}", self.hostname)];
        for (i, ext) in self.extensions.iter().enumerate() {
            if i == self.extensions.len() - 1 {
                lines.push(format!("250 {}", ext)); // Last line uses space instead of dash
            } else {
                lines.push(format!("250-{}", ext));
            }
        }
        lines
    }
} 