use async_trait::async_trait;
use rustycog_core::error::DomainError;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::{Permission, ProjectMemberRolePermission, Resource, RolePermission};
use crate::port::{
    PermissionReadRepository, ProjectMemberRolePermissionRepository,
    ResourceRepository, RolePermissionRepository,
};

#[async_trait]
pub trait PermissionService: Send + Sync {
    /// Get a permission by level
    async fn get_permission_by_level(&self, level: &str) -> Result<Permission, DomainError>;

    /// Get all available permissions
    async fn get_all_permissions(&self) -> Result<Vec<Permission>, DomainError>;

    /// Get a resource by id
    async fn get_resource(&self, resource_id: &str) -> Result<Resource, DomainError>;

    /// Get all resources
    async fn get_all_resources(&self) -> Result<Vec<Resource>, DomainError>;

    /// Create a resource for a component
    async fn create_component_resource(&self, component_type: &str) -> Result<Resource, DomainError>;

    /// Delete a resource by id
    async fn delete_resource(&self, resource_id: &str) -> Result<(), DomainError>;

    /// Get or create a role_permission for a project+resource+permission combination
    async fn get_or_create_role_permission(
        &self,
        project_id: Uuid,
        resource_name: &str,
        permission_level: &str,
    ) -> Result<RolePermission, DomainError>;

    /// Get role_permissions for a project
    async fn get_role_permissions_for_project(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError>;

    /// Grant a permission to a member
    async fn grant_permission_to_member(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError>;

    /// Revoke a permission from a member
    async fn revoke_permission_from_member(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<(), DomainError>;

    /// Revoke all permissions from a member
    async fn revoke_all_permissions_from_member(&self, member_id: &Uuid) -> Result<(), DomainError>;
}

pub struct PermissionServiceImpl<PR, RR, RPR, PMRPR>
where
    PR: PermissionReadRepository,
    RR: ResourceRepository,
    RPR: RolePermissionRepository,
    PMRPR: ProjectMemberRolePermissionRepository,
{
    permission_repo: Arc<PR>,
    resource_repo: Arc<RR>,
    role_permission_repo: Arc<RPR>,
    member_role_permission_repo: Arc<PMRPR>,
}

impl<PR, RR, RPR, PMRPR> PermissionServiceImpl<PR, RR, RPR, PMRPR>
where
    PR: PermissionReadRepository,
    RR: ResourceRepository,
    RPR: RolePermissionRepository,
    PMRPR: ProjectMemberRolePermissionRepository,
{
    pub fn new(
        permission_repo: Arc<PR>,
        resource_repo: Arc<RR>,
        role_permission_repo: Arc<RPR>,
        member_role_permission_repo: Arc<PMRPR>,
    ) -> Self {
        Self {
            permission_repo,
            resource_repo,
            role_permission_repo,
            member_role_permission_repo,
        }
    }
}

#[async_trait]
impl<PR, RR, RPR, PMRPR> PermissionService for PermissionServiceImpl<PR, RR, RPR, PMRPR>
where
    PR: PermissionReadRepository,
    RR: ResourceRepository,
    RPR: RolePermissionRepository,
    PMRPR: ProjectMemberRolePermissionRepository,
{
    async fn get_permission_by_level(&self, level: &str) -> Result<Permission, DomainError> {
        self.permission_repo
            .find_by_level(level)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Permission", level))
    }

    async fn get_all_permissions(&self) -> Result<Vec<Permission>, DomainError> {
        self.permission_repo.find_all().await
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Resource, DomainError> {
        self.resource_repo
            .find_by_id(resource_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Resource", resource_id))
    }

    async fn get_all_resources(&self) -> Result<Vec<Resource>, DomainError> {
        self.resource_repo.find_all().await
    }

    async fn create_component_resource(&self, component_type: &str) -> Result<Resource, DomainError> {
        self.resource_repo.create_for_component(component_type).await
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), DomainError> {
        self.resource_repo.delete_by_id(resource_id).await
    }

    async fn get_or_create_role_permission(
        &self,
        project_id: Uuid,
        resource_name: &str,
        permission_level: &str,
    ) -> Result<RolePermission, DomainError> {
        // Try to find existing
        if let Some(existing) = self
            .role_permission_repo
            .find_by_project_resource_permission(&project_id, resource_name, permission_level)
            .await?
        {
            return Ok(existing);
        }

        // Get permission and resource
        let permission = self.get_permission_by_level(permission_level).await?;
        let resource = self.get_resource(resource_name).await?;

        // Create new role_permission
        let role_perm = RolePermission {
            id: None,
            name: None,
            project_id,
            permission,
            resource,
            created_at: None,
        };

        self.role_permission_repo.create(&role_perm).await
    }

    async fn get_role_permissions_for_project(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError> {
        self.role_permission_repo.find_by_project(project_id).await
    }

    async fn grant_permission_to_member(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        self.member_role_permission_repo
            .grant(member_id, role_permission_id)
            .await
    }

    async fn revoke_permission_from_member(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<(), DomainError> {
        self.member_role_permission_repo
            .revoke(member_id, role_permission_id)
            .await
    }

    async fn revoke_all_permissions_from_member(&self, member_id: &Uuid) -> Result<(), DomainError> {
        self.member_role_permission_repo
            .revoke_all_for_member(member_id)
            .await
    }
}

