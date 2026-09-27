use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::RolePermission;
use rustycog::core::error::DomainError;
use uuid::Uuid;

/// Synchronous Hive → IAM organization-signer configuration (ADR-0306).
///
/// Distinct from [`ExternalProviderClient`] (GitHub/GitLab). Secrets never
/// appear in Hive DB or domain events — only `signing_profile_id` / status.
#[async_trait]
pub trait IamOrganizationSignerClient: Send + Sync {
    /// Configure (or replace) an organization signer profile in IAM.
    async fn configure_organization_signer(
        &self,
        org_id: Uuid,
        request: &ConfigureOrganizationSignerRequest,
    ) -> Result<OrganizationSignerResponse, DomainError>;

    /// Test that IAM can reach the configured signing backend.
    async fn test_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError>;

    /// Rotate the organization signing key (new kid; old becomes retiring).
    async fn rotate_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError>;

    /// Disable the organization signer (fall back to platform key).
    async fn disable_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError>;
}

/// Request body for ConfigureOrganizationSigner (no raw cloud secrets in Hive events).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureOrganizationSignerRequest {
    pub provider_type: String,
    pub provider_key_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_ref: Option<String>,
    pub public_key: String,
    pub org_slug: String,
}

/// UX metadata returned by IAM — never contains private key material.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrganizationSignerResponse {
    pub signing_profile_id: Uuid,
    pub kid: String,
    pub status: String,
    pub issuer: String,
}

/// Opaque s2s / KMS credential obtained via [`WorkloadIdentity`].
/// Never logged; never stored in Hive DB or domain events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadCredential {
    pub secret: String,
}

/// Port for obtaining s2s / KMS credentials without baking SA key JSON into Hive DB.
///
/// Preference order (ADR-0307): OIDC WIF, then X509/mTLS, then
/// `StaticCredential` last (OpenBao / config map). SPIFFE/SPIRE is not
/// a required adapter.
#[async_trait]
pub trait WorkloadIdentity: Send + Sync {
    /// Resolve a named credential reference (config key, OpenBao path, …).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the reference cannot be resolved.
    async fn resolve(&self, credential_ref: &str) -> Result<WorkloadCredential, DomainError>;
}

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
