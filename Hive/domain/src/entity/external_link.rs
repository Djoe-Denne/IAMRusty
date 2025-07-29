use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::DomainError;
use super::ProviderType;

/// External link entity representing connection between organization and external provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalLink {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub provider_id: Uuid,
    pub provider_config: Value,
    pub sync_enabled: bool,
    pub sync_settings: Value,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<SyncStatus>,
    pub sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sync status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncStatus {
    Success,
    Failed,
    Partial,
}

impl ExternalLink {
    /// Create a new external link
    pub fn new(
        organization_id: Uuid,
        provider_id: Uuid,
        provider_config: Value,
        sync_settings: Option<Value>,
    ) -> Result<Self, DomainError> {
        Self::validate_provider_config(&provider_config)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            organization_id,
            provider_id,
            provider_config,
            sync_enabled: false,
            sync_settings: sync_settings.unwrap_or_else(|| serde_json::json!({})),
            last_sync_at: None,
            last_sync_status: None,
            sync_error: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update provider configuration
    pub fn update_provider_config(&mut self, new_config: Value) -> Result<(), DomainError> {
        Self::validate_provider_config(&new_config)?;
        self.provider_config = new_config;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update sync settings
    pub fn update_sync_settings(&mut self, new_settings: Value) {
        self.sync_settings = new_settings;
        self.updated_at = Utc::now();
    }

    /// Enable synchronization
    pub fn enable_sync(&mut self) {
        self.sync_enabled = true;
        self.updated_at = Utc::now();
    }

    /// Disable synchronization
    pub fn disable_sync(&mut self) {
        self.sync_enabled = false;
        self.updated_at = Utc::now();
    }

    /// Record a successful sync
    pub fn record_sync_success(&mut self) {
        self.last_sync_at = Some(Utc::now());
        self.last_sync_status = Some(SyncStatus::Success);
        self.sync_error = None;
        self.updated_at = Utc::now();
    }

    /// Record a failed sync
    pub fn record_sync_failure(&mut self, error: String) {
        self.last_sync_at = Some(Utc::now());
        self.last_sync_status = Some(SyncStatus::Failed);
        self.sync_error = Some(error);
        self.updated_at = Utc::now();
    }

    /// Record a partial sync
    pub fn record_sync_partial(&mut self, error: Option<String>) {
        self.last_sync_at = Some(Utc::now());
        self.last_sync_status = Some(SyncStatus::Partial);
        self.sync_error = error;
        self.updated_at = Utc::now();
    }

    /// Check if sync is currently enabled
    pub fn is_sync_enabled(&self) -> bool {
        self.sync_enabled
    }

    /// Check if last sync was successful
    pub fn is_last_sync_successful(&self) -> bool {
        matches!(self.last_sync_status, Some(SyncStatus::Success))
    }

    /// Check if the link has ever been synced
    pub fn has_been_synced(&self) -> bool {
        self.last_sync_at.is_some()
    }

    /// Get sync health status
    pub fn get_sync_health(&self) -> SyncHealth {
        match (&self.last_sync_status, &self.sync_error) {
            (Some(SyncStatus::Success), _) => SyncHealth::Healthy,
            (Some(SyncStatus::Partial), _) => SyncHealth::Warning,
            (Some(SyncStatus::Failed), _) => SyncHealth::Error,
            (None, _) => SyncHealth::Unknown,
        }
    }

    /// Validate provider configuration
    fn validate_provider_config(config: &Value) -> Result<(), DomainError> {
        if !config.is_object() {
            return Err(DomainError::invalid_input(
                "Provider config must be a JSON object"
            ));
        }

        let config_obj = config.as_object().unwrap();
        if config_obj.is_empty() {
            return Err(DomainError::invalid_input(
                "Provider config cannot be empty"
            ));
        }

        Ok(())
    }
}

/// Sync health enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncHealth {
    Healthy,
    Warning,
    Error,
    Unknown,
}

impl SyncStatus {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncStatus::Success => "success",
            SyncStatus::Failed => "failed",
            SyncStatus::Partial => "partial",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "success" => Ok(SyncStatus::Success),
            "failed" => Ok(SyncStatus::Failed),
            "partial" => Ok(SyncStatus::Partial),
            _ => Err(DomainError::invalid_input(&format!(
                "Unknown sync status: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_external_link() {
        let org_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();
        let config = serde_json::json!({
            "org_name": "test-org",
            "access_token": "token123"
        });

        let link = ExternalLink::new(org_id, provider_id, config.clone(), None);

        assert!(link.is_ok());
        let link = link.unwrap();
        assert_eq!(link.organization_id, org_id);
        assert_eq!(link.provider_id, provider_id);
        assert_eq!(link.provider_config, config);
        assert!(!link.sync_enabled);
        assert!(!link.has_been_synced());
    }

    #[test]
    fn test_validate_provider_config() {
        let org_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();

        // Empty config should fail
        let result = ExternalLink::new(org_id, provider_id, serde_json::json!({}), None);
        assert!(result.is_err());

        // Non-object config should fail
        let result = ExternalLink::new(org_id, provider_id, serde_json::json!("invalid"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_enable_disable_sync() {
        let org_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();
        let config = serde_json::json!({"key": "value"});

        let mut link = ExternalLink::new(org_id, provider_id, config, None).unwrap();

        assert!(!link.is_sync_enabled());

        link.enable_sync();
        assert!(link.is_sync_enabled());

        link.disable_sync();
        assert!(!link.is_sync_enabled());
    }

    #[test]
    fn test_record_sync_results() {
        let org_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();
        let config = serde_json::json!({"key": "value"});

        let mut link = ExternalLink::new(org_id, provider_id, config, None).unwrap();

        // Record success
        link.record_sync_success();
        assert!(link.has_been_synced());
        assert!(link.is_last_sync_successful());
        assert!(matches!(link.get_sync_health(), SyncHealth::Healthy));
        assert!(link.sync_error.is_none());

        // Record failure
        link.record_sync_failure("Connection failed".to_string());
        assert!(!link.is_last_sync_successful());
        assert!(matches!(link.get_sync_health(), SyncHealth::Error));
        assert_eq!(link.sync_error, Some("Connection failed".to_string()));

        // Record partial
        link.record_sync_partial(Some("Some items failed".to_string()));
        assert!(matches!(link.get_sync_health(), SyncHealth::Warning));
        assert_eq!(link.sync_error, Some("Some items failed".to_string()));
    }

    #[test]
    fn test_update_provider_config() {
        let org_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();
        let config = serde_json::json!({"key": "value"});

        let mut link = ExternalLink::new(org_id, provider_id, config, None).unwrap();
        let original_updated_at = link.updated_at;

        let new_config = serde_json::json!({"new_key": "new_value"});
        let result = link.update_provider_config(new_config.clone());

        assert!(result.is_ok());
        assert_eq!(link.provider_config, new_config);
        assert!(link.updated_at > original_updated_at);
    }

    #[test]
    fn test_sync_status_conversion() {
        assert_eq!(SyncStatus::Success.as_str(), "success");
        assert_eq!(SyncStatus::Failed.as_str(), "failed");
        assert_eq!(SyncStatus::Partial.as_str(), "partial");

        assert!(matches!(
            SyncStatus::from_str("success").unwrap(),
            SyncStatus::Success
        ));
        assert!(matches!(
            SyncStatus::from_str("FAILED").unwrap(),
            SyncStatus::Failed
        ));
        assert!(SyncStatus::from_str("invalid").is_err());
    }
} 