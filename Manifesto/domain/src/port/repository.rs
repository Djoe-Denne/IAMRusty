use async_trait::async_trait;
use rustycog_core::error::DomainError;
use uuid::Uuid;

use crate::entity::{
    Permission, Project, ProjectComponent, ProjectMember, ProjectMemberRolePermission, Resource,
    RolePermission,
};
use crate::value_objects::{MemberSource, OwnerType, ProjectStatus};

// =============================================================================
// Project Repository Ports
// =============================================================================

#[async_trait]
pub trait ProjectReadRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Project>, DomainError>;
    
    async fn find_by_owner(
        &self,
        owner_type: OwnerType,
        owner_id: &Uuid,
    ) -> Result<Vec<Project>, DomainError>;
    
    async fn list_with_filters(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Project>, DomainError>;
    
    async fn count(&self) -> Result<i64, DomainError>;
    
    async fn count_with_filters(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
    ) -> Result<i64, DomainError>;
}

#[async_trait]
pub trait ProjectWriteRepository: Send + Sync {
    async fn save(&self, project: &Project) -> Result<Project, DomainError>;
    
    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;
    
    async fn exists_by_id(&self, id: &Uuid) -> Result<bool, DomainError>;
}

pub trait ProjectRepository: ProjectReadRepository + ProjectWriteRepository + Send + Sync {}

// =============================================================================
// Component Repository Ports
// =============================================================================

#[async_trait]
pub trait ComponentReadRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ProjectComponent>, DomainError>;
    
    async fn find_by_project(&self, project_id: &Uuid) -> Result<Vec<ProjectComponent>, DomainError>;
    
    async fn find_by_project_and_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<Option<ProjectComponent>, DomainError>;
    
    async fn count_active_by_project(&self, project_id: &Uuid) -> Result<i64, DomainError>;
}

#[async_trait]
pub trait ComponentWriteRepository: Send + Sync {
    async fn save(&self, component: &ProjectComponent) -> Result<ProjectComponent, DomainError>;
    
    async fn delete(&self, id: &Uuid) -> Result<(), DomainError>;
    
    async fn exists_by_project_and_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<bool, DomainError>;
}

pub trait ComponentRepository: ComponentReadRepository + ComponentWriteRepository + Send + Sync {}

// =============================================================================
// Member Repository Ports
// =============================================================================

#[async_trait]
pub trait MemberReadRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ProjectMember>, DomainError>;
    
    async fn find_by_project(&self, project_id: &Uuid) -> Result<Vec<ProjectMember>, DomainError>;
    
    async fn find_by_project_and_user(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<ProjectMember>, DomainError>;
    
    async fn list_with_filters(
        &self,
        project_id: &Uuid,
        source: Option<MemberSource>,
        active_only: bool,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError>;
    
    async fn count_active(&self, project_id: &Uuid) -> Result<i64, DomainError>;
}

#[async_trait]
pub trait MemberWriteRepository: Send + Sync {
    async fn save(&self, member: &ProjectMember) -> Result<ProjectMember, DomainError>;
    
    async fn delete(&self, id: &Uuid) -> Result<(), DomainError>;
    
    async fn exists_by_project_and_user(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError>;
}

pub trait MemberRepository: MemberReadRepository + MemberWriteRepository + Send + Sync {}

// =============================================================================
// Permission Repository Ports (Read-only, seeded data)
// =============================================================================

#[async_trait]
pub trait PermissionReadRepository: Send + Sync {
    async fn find_by_level(&self, level: &str) -> Result<Option<Permission>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Permission>, DomainError>;
}

// =============================================================================
// Resource Repository Ports
// =============================================================================

#[async_trait]
pub trait ResourceReadRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Resource>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Resource>, DomainError>;
    /// Find a resource for a specific component instance by its UUID
    async fn find_by_component_id(&self, component_id: &Uuid) -> Result<Option<Resource>, DomainError>;
}

#[async_trait]
pub trait ResourceWriteRepository: Send + Sync {
    async fn create_for_component(&self, component_type: &str) -> Result<Resource, DomainError>;
    async fn delete_by_id(&self, id: &str) -> Result<(), DomainError>;
    /// Create a resource for a specific component instance (format: "component:{uuid}")
    async fn create_for_component_instance(&self, component_id: &Uuid) -> Result<Resource, DomainError>;
}

pub trait ResourceRepository: ResourceReadRepository + ResourceWriteRepository + Send + Sync {}

// =============================================================================
// RolePermission Repository Ports
// =============================================================================

#[async_trait]
pub trait RolePermissionReadRepository: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError>;
    async fn find_by_project(&self, project_id: &Uuid)
        -> Result<Vec<RolePermission>, DomainError>;
    async fn find_by_project_resource_permission(
        &self,
        project_id: &Uuid,
        resource_name: &str,
        permission_level: &str,
    ) -> Result<Option<RolePermission>, DomainError>;
}

#[async_trait]
pub trait RolePermissionWriteRepository: Send + Sync {
    async fn create(&self, role_permission: &RolePermission)
        -> Result<RolePermission, DomainError>;
    async fn delete(&self, id: &Uuid) -> Result<(), DomainError>;
}

pub trait RolePermissionRepository:
    RolePermissionReadRepository + RolePermissionWriteRepository + Send + Sync
{
}

// =============================================================================
// ProjectMemberRolePermission Repository Ports
// =============================================================================

#[async_trait]
pub trait ProjectMemberRolePermissionReadRepository: Send + Sync {
    async fn find_by_member(
        &self,
        member_id: &Uuid,
    ) -> Result<Vec<ProjectMemberRolePermission>, DomainError>;
}

#[async_trait]
pub trait ProjectMemberRolePermissionWriteRepository: Send + Sync {
    async fn grant(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError>;
    async fn revoke(&self, member_id: &Uuid, role_permission_id: &Uuid)
        -> Result<(), DomainError>;
    async fn revoke_all_for_member(&self, member_id: &Uuid) -> Result<(), DomainError>;
}

pub trait ProjectMemberRolePermissionRepository:
    ProjectMemberRolePermissionReadRepository
    + ProjectMemberRolePermissionWriteRepository
    + Send
    + Sync
{
}

