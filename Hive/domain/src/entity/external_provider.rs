use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use rustycog_core::error::DomainError;

/// External provider entity representing third-party provider configurations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalProvider {
    pub id: Uuid,
    pub provider_source: String,
    pub name: String,
    pub config_schema: Option<Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl ExternalProvider {
    /// Create a new external provider
    pub fn new(
        provider_source: String,
        name: String,
        config_schema: Option<Value>,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;

        Ok(Self {
            id: Uuid::new_v4(),
            provider_source,
            name,
            config_schema,
            is_active: true,
            created_at: Utc::now(),
        })
    }

    /// Update provider name
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        Self::validate_name(&new_name)?;
        self.name = new_name;
        Ok(())
    }

    /// Update config schema
    pub fn update_config_schema(&mut self, new_schema: Option<Value>) {
        self.config_schema = new_schema;
    }

    /// Activate the provider
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Deactivate the provider
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Validate provider name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input("Provider name cannot be empty"));
        }

        if name.len() > 100 {
            return Err(DomainError::invalid_input(
                "Provider name cannot be longer than 100 characters",
            ));
        }

        Ok(())
    }
}
