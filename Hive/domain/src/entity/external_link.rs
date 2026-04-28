use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use rustycog_core::error::DomainError;

/// External link entity representing connection between organization and external provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalLink {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: Option<String>,
    pub provider_id: Uuid,
    pub provider_source: Option<String>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Success,
    Failed,
    Partial,
}

impl ExternalLink {
    /// Create a new external link
    pub fn new(
        organization_id: Uuid,
        organization_name: Option<String>,
        provider_id: Uuid,
        provider_source: Option<String>,
        provider_config: Value,
        sync_settings: Option<Value>,
    ) -> Result<Self, DomainError> {
        Self::validate_provider_config(&provider_config)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            organization_id,
            organization_name,
            provider_id,
            provider_source,
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
    #[must_use]
    pub const fn is_sync_enabled(&self) -> bool {
        self.sync_enabled
    }

    /// Check if last sync was successful
    #[must_use]
    pub const fn is_last_sync_successful(&self) -> bool {
        matches!(self.last_sync_status, Some(SyncStatus::Success))
    }

    /// Check if the link has ever been synced
    #[must_use]
    pub const fn has_been_synced(&self) -> bool {
        self.last_sync_at.is_some()
    }

    /// Get sync health status
    #[must_use]
    pub const fn get_sync_health(&self) -> SyncHealth {
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
                "Provider config must be a JSON object",
            ));
        }

        let config_obj = config.as_object().unwrap();
        if config_obj.is_empty() {
            return Err(DomainError::invalid_input(
                "Provider config cannot be empty",
            ));
        }

        Ok(())
    }

    pub fn set_organization_name(&mut self, organization_name: String) {
        self.organization_name = Some(organization_name);
    }

    pub fn set_provider_source(&mut self, provider_source: String) {
        self.provider_source = Some(provider_source);
    }
}

/// Sync health enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncHealth {
    Healthy,
    Warning,
    Error,
    Unknown,
}

impl SyncStatus {
    /// Get string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Partial => "partial",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "partial" => Ok(Self::Partial),
            _ => Err(DomainError::invalid_input(&format!(
                "Unknown sync status: {s}"
            ))),
        }
    }
}
