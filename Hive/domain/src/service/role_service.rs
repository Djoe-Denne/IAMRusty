use uuid::Uuid;

use crate::{
    entity::*,
    error::DomainError,
    port::*,
};

/// Domain service for organization member management
pub struct RoleServiceImpl<MR, OR, RR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RR: RolePermissionRepository,
{
    member_repo: MR,
    organization_repo: OR,
    role_repo: RR,
}

#[async_trait::async_trait]
pub trait RoleService {
    async fn check_read_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
    async fn check_admin_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
}

impl<MR, OR, RR> RoleServiceImpl<MR, OR, RR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RR: RolePermissionRepository,
{
    /// Create a new member service
    pub fn new(
        member_repo: MR,
        organization_repo: OR,
        role_repo: RR,
    ) -> Self {
        Self {
            member_repo,
            organization_repo,
            role_repo,
        }
    }

    async fn check_permission(&self, organization_id: &Uuid, user_id: &Uuid, resource_type: &str, permission: &str) -> Result<(), DomainError> {
        let organization = self.organization_repo.find_by_id(organization_id).await.map_err(|e| DomainError::Internal { message: e.to_string() })?.ok_or(DomainError::EntityNotFound {
            entity_type: "Organization".to_string(),
            id: organization_id.to_string(),
        })?;

        let is_public = organization.settings.get("public").unwrap_or(&serde_json::Value::Bool(false)).as_bool().unwrap_or(false);

        let matching_roles = self.role_repo.find_by_organization_role_permission_and_resource(organization_id, resource_type, permission).await.map_err(|e| DomainError::Internal { message: e.to_string() })?;

        if is_public {
            return Ok(());
        }

        let member = self.member_repo.find_by_organization_and_user(organization_id, user_id).await.map_err(|e| DomainError::Internal { message: e.to_string() })?.ok_or(DomainError::EntityNotFound {
    }

}

#[async_trait::async_trait]
impl<MR, OR, RR> RoleService for RoleServiceImpl<MR, OR, RR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RR: RolePermissionRepository,
{
    async fn check_read_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError> {
        self.check_permission(organization_id, user_id, organization_role::permissions::READ_ORG)
    }

    async fn check_admin_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError> {
        self.check_permission(organization_id, user_id, organization_role::permissions::ADMIN_ORG)
    }
} 