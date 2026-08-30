use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::RolePermission;
use rustycog::core::error::DomainError;

/// Generic external provider service trait
#[async_trait]
pub trait ExternalProviderClient: Send + Sync {
    /// Validate provider configuration for the given source.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the configuration is invalid.
    async fn validate_config(
        &self,
        provider_source: &str,
        config: &serde_json::Value,
    ) -> Result<(), DomainError>;

    /// Test connectivity against the external provider.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider cannot be reached or rejects the request.
    async fn test_connection(
        &self,
        provider_source: &str,
        config: &serde_json::Value,
    ) -> Result<bool, DomainError>;

    /// Synchronize members from the external provider.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider call fails.
    async fn sync_members(
        &self,
        provider_source: &str,
        config: &serde_json::Value,
    ) -> Result<Vec<ExternalMember>, DomainError>;

    /// Fetch organization metadata from the external provider.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider call fails.
    async fn get_organization_info(
        &self,
        provider_source: &str,
        config: &serde_json::Value,
    ) -> Result<ExternalOrganizationInfo, DomainError>;

    /// List members from the external provider.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider call fails.
    async fn get_members(
        &self,
        provider_source: &str,
        config: &serde_json::Value,
    ) -> Result<Vec<ExternalMember>, DomainError>;

    /// Check whether a username is a member of the external organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider call fails.
    async fn is_member(
        &self,
        provider_source: &str,
        config: &serde_json::Value,
        username: &str,
    ) -> Result<bool, DomainError>;
}

// External provider data types

/// Member record returned by an external identity provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMember {
    pub external_id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub roles: Vec<RolePermission>,
    pub is_active: bool,
    pub provider_source: String,
}

/// Organization metadata returned by an external identity provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalOrganizationInfo {
    pub external_id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub member_count: Option<i32>,
    pub is_public: bool,
    pub provider_source: String,
}

/// Static descriptor of an external provider and its configuration schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalProviderInfo {
    pub name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
    pub supported_features: Vec<String>,
    pub provider_source: String,
}

// Permission types are now provided by rustycog-permission crate
