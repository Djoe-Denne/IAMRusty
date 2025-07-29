use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainError, ProviderType};

// Import events from hive-events crate
use hive_events::{
    OrganizationCreatedEvent, OrganizationUpdatedEvent, OrganizationDeletedEvent,
    MemberInvitedEvent, MemberJoinedEvent, MemberRemovedEvent,
    InvitationCreatedEvent, InvitationAcceptedEvent, InvitationExpiredEvent,
    ExternalLinkCreatedEvent, SyncJobStartedEvent, SyncJobCompletedEvent,
};

/// External provider service trait for GitHub integration
#[async_trait]
pub trait GitHubProviderService {
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, DomainError>;
    async fn sync_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
    async fn get_organization_info(&self, config: &serde_json::Value) -> Result<ExternalOrganizationInfo, DomainError>;
    async fn get_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
    async fn is_member(&self, config: &serde_json::Value, username: &str) -> Result<bool, DomainError>;
}

/// External provider service trait for GitLab integration
#[async_trait]
pub trait GitLabProviderService {
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, DomainError>;
    async fn sync_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
    async fn get_organization_info(&self, config: &serde_json::Value) -> Result<ExternalOrganizationInfo, DomainError>;
    async fn get_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
    async fn is_member(&self, config: &serde_json::Value, username: &str) -> Result<bool, DomainError>;
}

/// External provider service trait for Confluence integration
#[async_trait]
pub trait ConfluenceProviderService {
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, DomainError>;
    async fn sync_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
    async fn get_organization_info(&self, config: &serde_json::Value) -> Result<ExternalOrganizationInfo, DomainError>;
    async fn get_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
    async fn is_member(&self, config: &serde_json::Value, username: &str) -> Result<bool, DomainError>;
}

/// Generic external provider service trait
#[async_trait]
pub trait ExternalProviderService {
    fn provider_type(&self) -> ProviderType;
    async fn get_provider_info(&self) -> Result<ExternalProviderInfo, DomainError>;
    async fn validate_config(&self, config: &serde_json::Value) -> Result<(), DomainError>;
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, DomainError>;
    async fn sync_members(&self, config: &serde_json::Value) -> Result<Vec<ExternalMember>, DomainError>;
}

// External provider data types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMember {
    pub external_id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: Option<String>,
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