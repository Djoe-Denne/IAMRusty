use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{entity::organization_member_role_permission::OrganizationMemberRolePermission, DomainError, ProviderType, RolePermission};

/// Generic external provider service trait
#[async_trait]
pub trait ExternalProviderClient: Send + Sync {
    fn provider_type(&self) -> ProviderType;
    async fn get_provider_info(&self) -> Result<ExternalProviderInfo, DomainError>;
    async fn validate_config(&self, config: &serde_json::Value) -> Result<(), DomainError>;
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, DomainError>;
    async fn sync_members(
        &self,
        config: &serde_json::Value,
    ) -> Result<Vec<ExternalMember>, DomainError>;
    async fn get_organization_info(
        &self,
        config: &serde_json::Value,
    ) -> Result<ExternalOrganizationInfo, DomainError>;
    async fn get_members(
        &self,
        config: &serde_json::Value,
    ) -> Result<Vec<ExternalMember>, DomainError>;
    async fn is_member(
        &self,
        config: &serde_json::Value,
        username: &str,
    ) -> Result<bool, DomainError>;
}

// External provider data types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMember {
    pub external_id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub roles: Vec<RolePermission>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalOrganizationInfo {
    pub external_id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub member_count: Option<i32>,
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalProviderInfo {
    pub provider_type: ProviderType,
    pub name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
    pub supported_features: Vec<String>,
}

#[async_trait]
pub trait RoleEngine: Send + Sync {
    async fn derive_role(
        &self,
        roles: Vec<OrganizationMemberRolePermission>,
        organization_settings: serde_json::Value,
    ) -> Result<Vec<OrganizationMemberRolePermission>, DomainError>;
}
