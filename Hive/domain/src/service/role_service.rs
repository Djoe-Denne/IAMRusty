use chrono::Utc;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use crate::{
    entity::{OrganizationMemberRolePermission, RolePermission},
    port::{
        MemberRoleRepository, PermissionRepository, ResourceRepository, RolePermissionRepository,
    },
};
use rustycog::core::error::DomainError;

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
    /// Create default system roles for a new organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn create_default_roles(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError>;

    /// Delete all roles for an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn delete_organization_roles(&self, organization_id: &Uuid) -> Result<(), DomainError>;

    /// Find a role permission by resource type and permission.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if no matching role permission is found.
    async fn find_role_permissions(
        &self,
        resource_type: &str,
        permission: &str,
        role_permissions: Vec<RolePermission>,
    ) -> Result<RolePermission, DomainError>;

    /// Find role permissions by organization ID.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn find_role_permissions_by_organization(
        &self,
        organization_id: &Uuid,
        role_permissions: &Vec<RolePermission>,
    ) -> Result<Vec<RolePermission>, DomainError>;

    /// Add roles to a member.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member is missing or persistence fails.
    async fn add_roles(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        roles: Vec<RolePermission>,
    ) -> Result<Vec<OrganizationMemberRolePermission>, DomainError>;

    /// List all role-permission templates for an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn list_organization_roles(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError>;

    /// Get one role-permission template scoped to an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the role is missing, belongs to another organization, or persistence fails.
    async fn get_organization_role(
        &self,
        organization_id: &Uuid,
        role_id: &Uuid,
    ) -> Result<RolePermission, DomainError>;
}

impl<MOR, RR, PR, RPR> RoleServiceImpl<MOR, RR, PR, RPR>
where
    MOR: MemberRoleRepository,
    RR: ResourceRepository,
    PR: PermissionRepository,
    RPR: RolePermissionRepository,
{
    /// Create a new member service
    pub const fn new(
        member_role_repo: Arc<MOR>,
        resource_repo: Arc<RR>,
        permission_repo: Arc<PR>,
        role_permission_repo: Arc<RPR>,
    ) -> Self {
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
    /// Create default system roles for a new organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn create_default_roles(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError> {
        let permissions =
            self.permission_repo
                .find_all()
                .await
                .map_err(|e| DomainError::Internal {
                    message: e.to_string(),
                })?;

        let resources = self
            .resource_repo
            .find_all()
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        debug!("Permissions: {:?}", permissions.clone());
        debug!("Resources: {:?}", resources.clone());

        let mut roles = Vec::new();
        for permission in &permissions {
            for resource in &resources {
                let name = format!("{}:{}", resource.name, permission.level.to_str());
                debug!("Role name: {:?}", name);
                let role = RolePermission::new(
                    None,
                    Some(name),
                    *organization_id,
                    permission,
                    resource,
                    Some(Utc::now()),
                );
                let role = self
                    .role_permission_repo
                    .save(organization_id, &role)
                    .await
                    .map_err(|e| DomainError::Internal {
                        message: e.to_string(),
                    })?;
                roles.push(role);
            }
        }

        Ok(roles)
    }

    /// Delete all roles for an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn delete_organization_roles(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        self.member_role_repo
            .delete_by_organization(organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;
        Ok(())
    }

    /// Find a role permission by resource type and permission.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if no matching role permission is found.
    async fn find_role_permissions(
        &self,
        resource_type: &str,
        permission: &str,
        role_permissions: Vec<RolePermission>,
    ) -> Result<RolePermission, DomainError> {
        role_permissions
            .iter()
            .find(|role_permission| {
                role_permission.resource.name == resource_type
                    && role_permission.permission.level.to_str() == permission
            })
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "RolePermission",
                    &format!("resource_type={resource_type}, permission={permission}"),
                )
            })
            .cloned()
    }

    /// Add roles to a member.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member is missing or persistence fails.
    async fn add_roles(
        &self,
        organization_id: &Uuid,
        member_id: &Uuid,
        roles: Vec<RolePermission>,
    ) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        let mut new_roles = Vec::new();
        for role in roles {
            let new_role = OrganizationMemberRolePermission::new(
                None,
                organization_id,
                member_id,
                &role,
                Utc::now(),
            );
            new_roles.push(self.member_role_repo.save(&new_role).await.map_err(|_| {
                DomainError::BusinessRuleViolation {
                    rule: "Trying to add roles to a unexisting member".to_string(),
                }
            })?);
        }
        Ok(new_roles)
    }

    /// Find role permissions by organization ID.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn find_role_permissions_by_organization(
        &self,
        organization_id: &Uuid,
        role_permissions: &Vec<RolePermission>,
    ) -> Result<Vec<RolePermission>, DomainError> {
        let roles = self
            .role_permission_repo
            .find_by_organization_roles(organization_id, role_permissions)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        Ok(roles)
    }

    /// List all role-permission templates for an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn list_organization_roles(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError> {
        self.role_permission_repo
            .find_by_organization(organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })
    }

    /// Get one role-permission template scoped to an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the role is missing, belongs to another organization, or persistence fails.
    async fn get_organization_role(
        &self,
        organization_id: &Uuid,
        role_id: &Uuid,
    ) -> Result<RolePermission, DomainError> {
        let role = self
            .role_permission_repo
            .find_by_id(role_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?
            .ok_or_else(|| DomainError::entity_not_found("RolePermission", &role_id.to_string()))?;

        if role.organization_id != *organization_id {
            return Err(DomainError::entity_not_found(
                "RolePermission",
                &role_id.to_string(),
            ));
        }

        Ok(role)
    }
}
