use chrono::{DateTime, Utc};
use tracing::debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::{entity::*, port::*};
use rustycog_core::error::DomainError;

/// Domain service for organization member management
pub struct RoleServiceImpl<MOR, RR, PR, RPR>
where
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RPR: RolePermissionRepository,
{
    member_role_repo: Arc<MOR>,
    resource_repo: Arc<RR>,
    permission_repo: Arc<PR>,
    role_permission_repo: Arc<RPR>,
}

#[async_trait::async_trait]
pub trait RoleService: Send + Sync {
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

impl<MOR, RR, PR, RPR> RoleServiceImpl<MOR, RR, PR, RPR>
where
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RPR: RolePermissionRepository,
{
    /// Create a new member service
    pub fn new(member_role_repo: Arc<MOR>, resource_repo: Arc<RR>, permission_repo: Arc<PR>, role_permission_repo: Arc<RPR>) -> Self {
        Self {
            member_role_repo,
            resource_repo,
            permission_repo,
            role_permission_repo,
        }
    }

}

#[async_trait::async_trait]
impl<MOR, RR, PR, RPR> RoleService for RoleServiceImpl<MOR, RR, PR, RPR>
where
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RPR: RolePermissionRepository,
{
    async fn create_default_roles(&self, organization_id: &Uuid) -> Result<Vec<RolePermission>, DomainError> {
        let permissions = self.permission_repo.find_all().await.map_err(|e| DomainError::Internal {
            message: e.to_string(),
        })?;

        let resources = self.resource_repo.find_all().await.map_err(|e| DomainError::Internal {
            message: e.to_string(),
        })?;
        
        debug!("Permissions: {:?}", permissions.clone());
        debug!("Resources: {:?}", resources.clone());

        let mut roles = Vec::new();
        for permission in &permissions {
            for resource in &resources {
                let name = format!("{}:{}", resource.name, permission.level.to_str());
                debug!("Role name: {:?}", name);
                let role = RolePermission::new(None, Some(name), *organization_id, permission, resource, Some(Utc::now()));
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
        role_permissions.iter().find(|role_permission| role_permission.resource.name == resource_type && role_permission.permission.level.to_str() == permission).ok_or(DomainError::entity_not_found(
            "RolePermission",
            &format!("resource_type={}, permission={}", resource_type, permission),
        )).cloned()
    }

    async fn add_roles(&self, organization_id: &Uuid, member_id: &Uuid, roles: Vec<RolePermission>) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        let mut new_roles = Vec::new();
        for role in roles {
            let new_role = OrganizationMemberRolePermission::new(None, organization_id, member_id, &role, Utc::now());
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