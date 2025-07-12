//! SMS communication adapter

use async_trait::async_trait;
use telegraph_domain::{DomainError, SmsService};
use aws_sdk_sns::{Client as SnsClient, Config as SnsConfig};
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use tracing::{info, error, debug};

/// SMS adapter configuration
#[derive(Debug, Clone)]
pub struct SmsConfig {
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub endpoint_url: Option<String>,
}

impl Default for SmsConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            endpoint_url: None,
        }
    }
}

/// SMS adapter using AWS SNS
pub struct SmsAdapter {
    client: SnsClient,
    config: SmsConfig,
}

impl SmsAdapter {
    /// Create a new SMS adapter with configuration
    pub async fn new(config: SmsConfig) -> Result<Self, DomainError> {
        let client = Self::create_sns_client(&config).await?;
        
        Ok(Self {
            client,
            config,
        })
    }
    
    /// Create a new SMS adapter with default configuration (for testing)
    pub fn new_default() -> Self {
        Self {
            client: Self::create_test_client(),
            config: SmsConfig::default(),
        }
    }
    
    /// Create SNS client from configuration
    async fn create_sns_client(config: &SmsConfig) -> Result<SnsClient, DomainError> {
        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest());

        // Set region
        aws_config_builder = aws_config_builder.region(Region::new(config.region.clone()));

        // Set endpoint if using localstack or custom endpoint
        if let Some(ref endpoint_url) = config.endpoint_url {
            aws_config_builder = aws_config_builder.endpoint_url(endpoint_url);
        }

        // Set credentials if provided
        if let (Some(ref access_key), Some(ref secret_key)) = (&config.access_key_id, &config.secret_access_key) {
            let credentials = Credentials::new(
                access_key,
                secret_key,
                config.session_token.clone(),
                None,
                "telegraph-sms",
            );
            aws_config_builder = aws_config_builder.credentials_provider(credentials);
        }

        let aws_config = aws_config_builder.load().await;
        let sns_config = SnsConfig::from(&aws_config);
        let client = SnsClient::from_conf(sns_config);

        Ok(client)
    }
    
    /// Create a test client that doesn't actually send SMS
    fn create_test_client() -> SnsClient {
        // Create a client with default configuration for testing
        let config = aws_sdk_sns::Config::builder()
            .region(Region::new("us-east-1"))
            .build();
        SnsClient::from_conf(config)
    }
}

#[async_trait]
impl SmsService for SmsAdapter {
    async fn send_sms(
        &self,
        to: &str,
        message: &str,
    ) -> Result<String, DomainError> {
        info!(
            to = to,
            message_length = message.len(),
            "Sending SMS via AWS SNS"
        );
        
        // Validate message length (SMS has character limits)
        if message.len() > 1600 {
            return Err(DomainError::invalid_message("SMS message too long (max 1600 characters)".to_string()));
        }
        
        // Send SMS via SNS
        match self.client
            .publish()
            .phone_number(to)
            .message(message)
            .send()
            .await
        {
            Ok(response) => {
                let message_id = response.message_id().unwrap_or("unknown").to_string();
                info!(
                    message_id = %message_id,
                    to = to,
                    "SMS sent successfully via AWS SNS"
                );
                Ok(message_id)
            }
            Err(e) => {
                error!(
                    error = %e,
                    to = to,
                    message_length = message.len(),
                    "Failed to send SMS via AWS SNS"
                );
                Err(DomainError::InfrastructureError(format!("Failed to send SMS: {}", e)))
            }
        }
    }
    
    fn validate_phone_number(&self, phone: &str) -> Result<(), DomainError> {
        if phone.len() >= 10 && phone.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == ' ') {
            Ok(())
        } else {
            Err(DomainError::invalid_message("Invalid phone number format".to_string()))
        }
    }
    
    async fn health_check(&self) -> Result<(), DomainError> {
        debug!("Performing SMS service health check");
        
        // Test SNS connection by listing SMS attributes
        match self.client.get_sms_attributes().send().await {
            Ok(_) => {
                debug!("✅ SMS service health check passed - SNS connection successful");
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    "❌ SMS service health check failed - SNS connection error"
                );
                Err(DomainError::InfrastructureError(format!("SNS health check error: {}", e)))
            }
        }
    }
} 