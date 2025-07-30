use async_trait::async_trait;
use chrono::naive::serde;
use uuid::Uuid;

use crate::{entity::*, error::DomainError};

/// Repository port for Organization entities
#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    /// Find organization by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Organization>, DomainError>;

    /// Find organization by slug
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Organization>, DomainError>;

    /// Find organizations by owner user ID
    async fn find_by_owner(&self, owner_user_id: &Uuid) -> Result<Vec<Organization>, DomainError>;

    /// Find organizations where user has any role
    async fn find_by_user_membership(
        &self,
        user_id: &Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError>;

    /// Search organizations by name
    async fn search_by_name(
        &self,
        name_pattern: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError>;

    /// Check if organization exists by slug
    async fn exists_by_slug(&self, slug: &str) -> Result<bool, DomainError>;

    /// Save organization (create or update)
    async fn save(&self, organization: &Organization) -> Result<Organization, DomainError>;

    /// Delete organization by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Count total organizations
    async fn count(&self) -> Result<i64, DomainError>;
}

/// Repository port for OrganizationMember entities
#[async_trait]
pub trait OrganizationMemberRepository: Send + Sync {
    /// Find member by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationMember>, DomainError>;

    /// Find member by organization and user ID
    async fn find_by_organization_and_user(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<OrganizationMember>, DomainError>;

    /// Find all members of an organization
    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<OrganizationMember>, DomainError>;

    /// Find all organizations a user is a member of
    async fn find_by_user(&self, user_id: &Uuid) -> Result<Vec<OrganizationMember>, DomainError>;

    /// Find members by status in an organization
    async fn find_by_organization_and_status(
        &self,
        organization_id: &Uuid,
        status: &MemberStatus,
    ) -> Result<Vec<OrganizationMember>, DomainError>;

    /// Find members by role in an organization
    async fn find_by_organization_and_role(
        &self,
        organization_id: &Uuid,
        role_id: &Uuid,
    ) -> Result<Vec<OrganizationMember>, DomainError>;

    /// Check if user is a member of organization
    async fn is_member(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<bool, DomainError>;

    /// Save member (create or update)
    async fn save(&self, member: &OrganizationMember) -> Result<OrganizationMember, DomainError>;

    /// Delete member by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Delete members by organization ID
    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError>;

    /// Count members in organization
    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError>;

    /// Count active members in organization
    async fn count_active_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<i64, DomainError>;
}

/// Repository port for Permission entities
#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Permission>, DomainError>;
    async fn find_by_level(
        &self,
        level: &PermissionLevel,
    ) -> Result<Option<Permission>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Permission>, DomainError>;
    async fn save(&self, permission: &Permission) -> Result<Permission, DomainError>;
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;
}

/// Repository port for Resource entities
#[async_trait]
pub trait ResourceRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Resource>, DomainError>;
    async fn find_by_type(
        &self,
        resource_type: &String,
    ) -> Result<Option<Resource>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Resource>, DomainError>;
    async fn save(&self, resource: &Resource) -> Result<Resource, DomainError>;
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;
}

/// Repository port for RolePermission entities
#[async_trait]
pub trait RolePermissionRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError>;
    async fn find_by_organization_resource_permission(
        &self,
        organization_id: &Uuid,
        role_permission: &RolePermission,
    ) -> Result<Vec<RolePermission>, DomainError>;
    async fn save(&self, organization_id: &Uuid, role_permission: &RolePermission) -> Result<RolePermission, DomainError>;
    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError>;
}

/// Repository port for MemberRole entities
#[async_trait]
pub trait MemberRoleRepository: Send + Sync {
    async fn find_by_organization_member(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
    ) -> Result<Vec<OrganizationMemberRolePermission>, DomainError>;
    async fn save(
        &self,
        member_role: &OrganizationMemberRolePermission,
    ) -> Result<OrganizationMemberRolePermission, DomainError>;
    async fn delete_by_organization_member(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
    ) -> Result<(), DomainError>;
    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError>;
}

/// Repository port for OrganizationInvitation entities
#[async_trait]
pub trait OrganizationInvitationRepository: Send + Sync {
    /// Find invitation by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationInvitation>, DomainError>;

    /// Find invitation by token
    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<OrganizationInvitation>, DomainError>;

    /// Find invitations by organization
    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError>;

    /// Find invitations by aggregate id
    async fn find_by_aggregate_id(&self, aggregate_id: &str) -> Result<Vec<OrganizationInvitation>, DomainError>;

    /// Find pending invitations by organization and aggregate id
    async fn find_by_organization_and_aggregate_id_status(
        &self,
        organization_id: &Uuid,
        aggregate_id: &str,
        status: &InvitationStatus,
    ) -> Result<Option<OrganizationInvitation>, DomainError>;

    /// Find invitations by status
    async fn find_by_status(
        &self,
        status: &InvitationStatus,
    ) -> Result<Vec<OrganizationInvitation>, DomainError>;

    /// Find expired invitations
    async fn find_expired(&self) -> Result<Vec<OrganizationInvitation>, DomainError>;

    /// Save invitation (create or update)
    async fn save(
        &self,
        invitation: &OrganizationInvitation,
    ) -> Result<OrganizationInvitation, DomainError>;

    /// Delete invitation by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Count invitations by organization
    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError>;

    /// Count pending invitations by organization
    async fn count_pending_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<i64, DomainError>;
}

/// Repository port for ExternalProvider entities
#[async_trait]
pub trait ExternalProviderRepository: Send + Sync {
    /// Find provider by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalProvider>, DomainError>;

    /// Find provider by type
    async fn find_by_type(
        &self,
        provider_type: &ProviderType,
    ) -> Result<Option<ExternalProvider>, DomainError>;

    /// Find all providers
    async fn find_all(&self) -> Result<Vec<ExternalProvider>, DomainError>;

    /// Find active providers
    async fn find_active(&self) -> Result<Vec<ExternalProvider>, DomainError>;

    /// Save provider (create or update)
    async fn save(&self, provider: &ExternalProvider) -> Result<ExternalProvider, DomainError>;

    /// Delete provider by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;
}

/// Repository port for ExternalLink entities
#[async_trait]
pub trait ExternalLinkRepository: Send + Sync {
    /// Find link by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalLink>, DomainError>;

    /// Find links by organization
    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<ExternalLink>, DomainError>;

    /// Find link by organization and provider
    async fn find_by_organization_and_provider(
        &self,
        organization_id: &Uuid,
        provider_id: &Uuid,
    ) -> Result<Option<ExternalLink>, DomainError>;

    /// Find links with sync enabled
    async fn find_sync_enabled(&self) -> Result<Vec<ExternalLink>, DomainError>;

    /// Find links that need sync (enabled and not recently synced)
    async fn find_needing_sync(&self, max_age_hours: i64)
        -> Result<Vec<ExternalLink>, DomainError>;

    /// Save link (create or update)
    async fn save(&self, link: &ExternalLink) -> Result<ExternalLink, DomainError>;

    /// Delete link by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Count links by organization
    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError>;
}

/// Repository port for SyncJob entities
#[async_trait]
pub trait SyncJobRepository: Send + Sync {
    /// Find job by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<SyncJob>, DomainError>;

    /// Find jobs by external link
    async fn find_by_external_link(&self, link_id: &Uuid) -> Result<Vec<SyncJob>, DomainError>;

    /// Find jobs by organization
    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<SyncJob>, DomainError>;

    /// Find jobs by status
    async fn find_by_status(&self, status: &SyncJobStatus) -> Result<Vec<SyncJob>, DomainError>;

    /// Find running jobs
    async fn find_running(&self) -> Result<Vec<SyncJob>, DomainError>;

    /// Find running jobs for external link
    async fn find_running_by_external_link(
        &self,
        link_id: &Uuid,
    ) -> Result<Vec<SyncJob>, DomainError>;

    /// Find recent jobs (last N days)
    async fn find_recent(&self, days: i64) -> Result<Vec<SyncJob>, DomainError>;

    /// Save job (create or update)
    async fn save(&self, job: &SyncJob) -> Result<SyncJob, DomainError>;

    /// Delete job by ID
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Count jobs by external link
    async fn count_by_external_link(&self, link_id: &Uuid) -> Result<i64, DomainError>;

    /// Count running jobs
    async fn count_running(&self) -> Result<i64, DomainError>;
}
