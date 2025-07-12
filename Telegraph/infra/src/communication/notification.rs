//! Push notification communication adapter

use async_trait::async_trait;
use telegraph_domain::{DomainError, NotificationService};
use std::collections::HashMap;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{info, error, debug};

/// Notification adapter configuration
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    pub fcm_server_key: String,
    pub fcm_project_id: String,
    pub apns_key_id: Option<String>,
    pub apns_team_id: Option<String>,
    pub apns_bundle_id: Option<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            fcm_server_key: "".to_string(),
            fcm_project_id: "".to_string(),
            apns_key_id: None,
            apns_team_id: None,
            apns_bundle_id: None,
        }
    }
}

/// Push notification adapter using FCM (Firebase Cloud Messaging)
pub struct NotificationAdapter {
    client: Client,
    config: NotificationConfig,
}

impl NotificationAdapter {
    /// Create a new notification adapter with configuration
    pub fn new(config: NotificationConfig) -> Result<Self, DomainError> {
        let client = Client::new();
        
        Ok(Self {
            client,
            config,
        })
    }
    
    /// Create a new notification adapter with default configuration (for testing)
    pub fn new_default() -> Self {
        Self {
            client: Client::new(),
            config: NotificationConfig::default(),
        }
    }
    
    /// Send notification via FCM
    async fn send_via_fcm(
        &self,
        device_token: &str,
        title: &str,
        body: &str,
        data: &HashMap<String, String>,
    ) -> Result<String, DomainError> {
        if self.config.fcm_server_key.is_empty() {
            return Err(DomainError::InfrastructureError("FCM server key not configured".to_string()));
        }
        
        let url = "https://fcm.googleapis.com/fcm/send";
        
        let payload = json!({
            "to": device_token,
            "notification": {
                "title": title,
                "body": body
            },
            "data": data
        });
        
        let response = self.client
            .post(url)
            .header("Authorization", format!("key={}", self.config.fcm_server_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("FCM request failed: {}", e)))?;
        
        if response.status().is_success() {
            let response_data: Value = response
                .json()
                .await
                .map_err(|e| DomainError::InfrastructureError(format!("Failed to parse FCM response: {}", e)))?;
            
            // Extract message ID from FCM response
            let message_id = response_data
                .get("results")
                .and_then(|r| r.get(0))
                .and_then(|r| r.get("message_id"))
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            Ok(message_id)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(DomainError::InfrastructureError(format!("FCM error: {}", error_text)))
        }
    }
}

#[async_trait]
impl NotificationService for NotificationAdapter {
    async fn send_notification(
        &self,
        device_token: Option<&str>,
        user_id: Option<&str>,
        title: &str,
        body: &str,
        data: &HashMap<String, String>,
    ) -> Result<String, DomainError> {
        info!(
            device_token = device_token,
            user_id = user_id,
            title = title,
            body = body,
            data_count = data.len(),
            "Sending push notification via FCM"
        );
        
        // We need a device token to send the notification
        let token = device_token.ok_or_else(|| {
            DomainError::invalid_message("Device token required for push notification".to_string())
        })?;
        
        // Send notification via FCM
        match self.send_via_fcm(token, title, body, data).await {
            Ok(message_id) => {
                info!(
                    message_id = %message_id,
                    device_token = device_token,
                    user_id = user_id,
                    "Push notification sent successfully via FCM"
                );
                Ok(message_id)
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_token = device_token,
                    user_id = user_id,
                    title = title,
                    "Failed to send push notification via FCM"
                );
                Err(e)
            }
        }
    }
    
    fn validate_device_token(&self, token: &str) -> Result<(), DomainError> {
        if token.len() > 10 {
            Ok(())
        } else {
            Err(DomainError::invalid_message("Invalid device token".to_string()))
        }
    }
    
    async fn health_check(&self) -> Result<(), DomainError> {
        debug!("Performing notification service health check");
        
        if self.config.fcm_server_key.is_empty() {
            error!("❌ Notification service health check failed - FCM server key not configured");
            return Err(DomainError::InfrastructureError("FCM server key not configured".to_string()));
        }
        
        // Simple health check - try to reach FCM servers
        let health_url = "https://fcm.googleapis.com/fcm/send";
        
        match self.client
            .post(health_url)
            .header("Authorization", format!("key={}", self.config.fcm_server_key))
            .header("Content-Type", "application/json")
            .json(&json!({"validate_only": true}))
            .send()
            .await
        {
            Ok(response) => {
                // Even if we get an error response, if we can reach FCM, the service is healthy
                debug!(
                    status = %response.status(),
                    "✅ Notification service health check passed - FCM servers reachable"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    "❌ Notification service health check failed - Cannot reach FCM servers"
                );
                Err(DomainError::InfrastructureError(format!("FCM health check error: {}", e)))
            }
        }
    }
} 