use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{entity::*, error::DomainError, port::*};

/// Domain service for organization member management
pub struct RoleServiceImpl<MR, OR, MOR, RR, PR, RE, RPR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RE: RoleEngine,
    RPR: RolePermissionRepository,
{
    member_repo: MR,
    organization_repo: OR,
    member_role_repo: MOR,
    resource_repo: RR,
    permission_repo: PR,
    role_engine: RE,
    role_permission_repo: RPR,
}

#[async_trait::async_trait]
pub trait RoleService: Send + Sync {
    /**
     * Check if a member has read permission for a resource
     * 
     * @param organization_id - The ID of the organization to check the permission for
     * @param member_id - The ID of the member to check the permission for
     * @param resource_type - The type of the resource to check the permission for
     */
    async fn check_read_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
    ) -> Result<bool, DomainError>;

    /**
     * Check if a member has admin permission for a resource
     * 
     * @param organization_id - The ID of the organization to check the permission for
     * @param member_id - The ID of the member to check the permission for
     * @param resource_type - The type of the resource to check the permission for
     */
    async fn check_admin_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
    ) -> Result<bool, DomainError>;

    /**
     * Check if a member has write permission for a resource
     * 
     * @param organization_id - The ID of the organization to check the permission for
     * @param member_id - The ID of the member to check the permission for
     * @param resource_type - The type of the resource to check the permission for
     */
    async fn check_write_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
    ) -> Result<bool, DomainError>;

    /**
     * Create default system roles for a new organization
     * 
     * @param organization_id - The ID of the organization to create the default roles for
     */
    async fn create_default_roles(&self, organization_id: &Uuid) -> Result<Vec<RolePermission>, DomainError>;

    /**
     * Delete all roles for an organization
     * 
     * @param organization_id - The ID of the organization to delete the roles for
     */
    async fn delete_organization_roles(&self, organization_id: &Uuid) -> Result<(), DomainError>;

    /**
     * Find a role permission by resource type and permission
     * 
     * @param resource_type - The type of the resource to find the role permission for
     * @param permission - The permission to find the role permission for
     * @param role_permissions - The list of role permissions to search in
     */
    async fn find_role_permissions(&self, resource_type: &str, permission: &str, role_permissions: Vec<RolePermission>) -> Result<RolePermission, DomainError>;

    /**
     * Find role permissions by organization ID
     * 
     * @param organization_id - The ID of the organization to find the role permissions for
     * @param role_permissions - The list of role permissions to search in
     */
    async fn find_role_permissions_by_organization(&self, organization_id: &Uuid, role_permissions: &Vec<RolePermission>) -> Result<Vec<RolePermission>, DomainError>;

    /**
     * Add roles to a member
     * 
     * @param organization_id - The ID of the organization to add the roles to
     * @param member_id - The ID of the member to add the roles to
     * @param roles - The roles to add to the member
     */
    async fn add_roles(&self, organization_id: &Uuid, member_id: &Uuid, roles: Vec<RolePermission>) -> Result<Vec<OrganizationMemberRolePermission>, DomainError>;
}

impl<MR, OR, MOR, RR, PR, RE, RPR> RoleServiceImpl<MR, OR, MOR, RR, PR, RE, RPR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RE: RoleEngine,
    RPR: RolePermissionRepository,
{
    /// Create a new member service
    pub fn new(member_repo: MR, organization_repo: OR, member_role_repo: MOR, resource_repo: RR, permission_repo: PR, role_engine: RE, role_permission_repo: RPR) -> Self {
        Self {
            member_repo,
            organization_repo,
            member_role_repo,
            resource_repo,
            permission_repo,
            role_engine,
            role_permission_repo,
        }
    }

    async fn check_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
        permission: &str,
    ) -> Result<bool, DomainError> {
        let organization = self
            .organization_repo
            .find_by_id(organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?
            .ok_or(DomainError::entity_not_found(
                "Organization",
                organization_id.to_string().as_str(),
            ))?;

        let member = self
            .member_repo
            .find_by_organization_and_user(organization_id, member_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?
            .ok_or(DomainError::entity_not_found(
                "Member",
                member_id.to_string().as_str(),
            ))?;

        let member_roles = self
            .member_role_repo
            .find_by_organization_member(organization_id, member_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        let roles = self
            .role_engine
            .derive_role(member_roles, organization.settings)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        if roles
            .iter()
            .any(|role| role.role_permission.resource.name == resource_type && role.role_permission.permission.level.as_str() == permission)
        {
            return Ok(true);
        }

        Ok(false)
    }
}

#[async_trait::async_trait]
impl<MR, OR, MOR, RR, PR, RE, RPR> RoleService for RoleServiceImpl<MR, OR, MOR, RR, PR, RE, RPR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RE: RoleEngine,
    RPR: RolePermissionRepository,
{
    async fn check_read_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
    ) -> Result<bool, DomainError> {
        self.check_permission(
            organization_id,
            member_id,
            resource_type,
            "read"
        ).await
    }

    async fn check_admin_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
    ) -> Result<bool, DomainError> {
        self.check_permission(
            organization_id,
            member_id,
            resource_type,
            "admin",
        ).await
    }

    async fn check_write_permission(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        resource_type: &str,
    ) -> Result<bool, DomainError> {
        self.check_permission(
            organization_id,
            member_id,
            resource_type,
            "write",
        ).await
    }

    async fn create_default_roles(&self, organization_id: &Uuid) -> Result<Vec<RolePermission>, DomainError> {
        let permissions = self.permission_repo.find_all().await.map_err(|e| DomainError::Internal {
            message: e.to_string(),
        })?;

        let resources = self.resource_repo.find_all().await.map_err(|e| DomainError::Internal {
            message: e.to_string(),
        })?;

        let mut roles = Vec::new();
        for permission in &permissions {
            for resource in &resources {
                let name = format!("{}:{}", resource.name, permission.level.as_str());
                let role = RolePermission::new(None, Some(name), None, permission, resource, Some(Utc::now()));
                let role = self.role_permission_repo.save(organization_id, &role).await.map_err(|e| DomainError::Internal {
                    message: e.to_string(),
                })?;
                roles.push(role);
            }
        }

        Ok(roles)
    }

    async fn delete_organization_roles(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        self.member_role_repo.delete_by_organization(organization_id).await.map_err(|e| DomainError::Internal {
            message: e.to_string(),
        })?;
        Ok(())
    }

    async fn find_role_permissions(&self, resource_type: &str, permission: &str, role_permissions: Vec<RolePermission>) -> Result<RolePermission, DomainError> {
        role_permissions.iter().find(|role_permission| role_permission.resource.name == resource_type && role_permission.permission.level.as_str() == permission).ok_or(DomainError::entity_not_found(
            "RolePermission",
            &format!("resource_type={}, permission={}", resource_type, permission),
        )).cloned()
    }

    async fn add_roles(&self, organization_id: &Uuid, member_id: &Uuid, roles: Vec<RolePermission>) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        let mut new_roles = Vec::new();
        for role in roles {
            let new_role = OrganizationMemberRolePermission::new(Uuid::new_v4(), organization_id, member_id, &role, Utc::now());
            new_roles.push(self.member_role_repo.save(&new_role).await.map_err(|e| DomainError::BusinessRuleViolation { rule: "Trying to add roles to a unexisting member".to_string() })?);
        }
        Ok(new_roles)
    }

    async fn find_role_permissions_by_organization(&self, organization_id: &Uuid, role_permissions: &Vec<RolePermission>) -> Result<Vec<RolePermission>, DomainError> {
        let roles = self.role_permission_repo.find_by_organization_roles(organization_id, &role_permissions).await.map_err(|e| DomainError::Internal {
            message: e.to_string(),
        })?;

        Ok(roles)
    }
}