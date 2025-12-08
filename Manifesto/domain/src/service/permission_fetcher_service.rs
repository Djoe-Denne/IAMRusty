use async_trait::async_trait;
use rustycog_core::error::DomainError;
use rustycog_permission::{Permission, PermissionsFetcher, ResourceId};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use crate::entity::project_member_role_permission::ProjectMemberRolePermission;
use crate::service::{MemberService, ProjectService};
use crate::value_objects::PermissionLevel;

/// Permission fetcher for project resources
pub struct ProjectPermissionFetcher<PS, MS>
where
    PS: ProjectService,
    MS: MemberService,
{
    project_service: Arc<PS>,
    member_service: Arc<MS>,
}

impl<PS, MS> ProjectPermissionFetcher<PS, MS>
where
    PS: ProjectService,
    MS: MemberService,
{
    pub fn new(project_service: Arc<PS>, member_service: Arc<MS>) -> Self {
        Self {
            project_service,
            member_service,
        }
    }
}

#[async_trait]
impl<PS, MS> PermissionsFetcher for ProjectPermissionFetcher<PS, MS>
where
    PS: ProjectService + Send + Sync,
    MS: MemberService + Send + Sync,
{
    async fn fetch_permissions(
        &self,
        user_id: Uuid,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        let mut permissions = vec![];
        if resource_ids.is_empty() {
            return Ok(permissions);
        }

        let project_id = resource_ids[0].id();
        debug!("Fetching permissions for user {} on project {}", user_id, project_id);

        // Get the project to check visibility
        let project = self.project_service.get_project(&project_id).await?;

        if project.is_public() {
            permissions.push(Permission::Read);
        }

        // Get the member's role on the project
        let member = match self.member_service.get_member(project_id, user_id).await {
            Ok(member) => member,
            Err(_) => {
                // User is not a member, no permissions
                return Ok(permissions);
            }
        };

        // Check if member is active
        if !member.is_active() {
            // User is not a member, no permissions
            return Ok(permissions);
        }

        // Get permissions for the "project" resource from member's role_permissions
        permissions.extend(get_permissions_for_resource(&member.role_permissions, "project"));

        debug!("User {} has permissions {:?} on project {}", user_id, permissions, project_id);

        Ok(permissions)
    }
}

/// Permission fetcher for component resources (delegates to project membership)
pub struct ComponentPermissionFetcher<PS, MS>
where
    PS: ProjectService,
    MS: MemberService,
{
    project_service: Arc<PS>,
    member_service: Arc<MS>,
}

impl<PS, MS> ComponentPermissionFetcher<PS, MS>
where
    PS: ProjectService,
    MS: MemberService,
{
    pub fn new(project_service: Arc<PS>, member_service: Arc<MS>) -> Self {
        Self {
            project_service,
            member_service,
        }
    }
}

#[async_trait]
impl<PS, MS> PermissionsFetcher for ComponentPermissionFetcher<PS, MS>
where
    PS: ProjectService + Send + Sync,
    MS: MemberService + Send + Sync,
{
    async fn fetch_permissions(
        &self,
        user_id: Uuid,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        let mut permissions = vec![];
        // Component permissions are based on project membership
        // resource_ids[0] should be the project_id
        if resource_ids.is_empty() {
            return Ok(permissions);
        }

        let project_id = resource_ids[0].id();
        debug!("Fetching component permissions for user {} on project {}", user_id, project_id);

        let project = self.project_service.get_project(&project_id).await?;

        if project.is_public() {
            permissions.push(Permission::Read);
        }

        let member = match self.member_service.get_member(project_id, user_id).await {
            Ok(member) => member,
            Err(_) => return Ok(permissions),
        };

        if !member.is_active() {
            return Ok(permissions);
        }

        // Get permissions for the "component" resource from member's role_permissions
        permissions.extend(get_permissions_for_resource(&member.role_permissions, "component"));

        Ok(permissions)
    }
}

/// Permission fetcher for member resources
pub struct MemberPermissionFetcher<PS, MS>
where
    PS: ProjectService,
    MS: MemberService,
{
    project_service: Arc<PS>,
    member_service: Arc<MS>,
}

impl<PS, MS> MemberPermissionFetcher<PS, MS>
where
    PS: ProjectService,
    MS: MemberService,
{
    pub fn new(project_service: Arc<PS>, member_service: Arc<MS>) -> Self {
        Self {
            project_service,
            member_service,
        }
    }
}

#[async_trait]
impl<PS, MS> PermissionsFetcher for MemberPermissionFetcher<PS, MS>
where
    PS: ProjectService + Send + Sync,
    MS: MemberService + Send + Sync,
{
    async fn fetch_permissions(
        &self,
        user_id: Uuid,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        let mut permissions = vec![];
        // Member management permissions are based on project membership
        if resource_ids.is_empty() {
            return Ok(permissions);
        }

        let project_id = resource_ids[0].id();
        debug!("Fetching member permissions for user {} on project {}", user_id, project_id);

        let project = self.project_service.get_project(&project_id).await?;

        if project.is_public() {
            permissions.push(Permission::Read);
        }

        let member = match self.member_service.get_member(project_id, user_id).await {
            Ok(member) => member,
            Err(_) => return Ok(permissions),
        };

        if !member.is_active() {
            return Ok(permissions);
        }

        // Get permissions for the "member" resource from member's role_permissions
        permissions.extend(get_permissions_for_resource(&member.role_permissions, "member"));

        Ok(permissions)
    }
}

/// Get permissions for a specific resource from the member's role_permissions
fn get_permissions_for_resource(
    role_permissions: &[ProjectMemberRolePermission],
    resource_name: &str,
) -> Vec<Permission> {
    // Find the highest permission level for the given resource
    // Use case-insensitive comparison since resource names in DB may be capitalized
    let highest_level = role_permissions
        .iter()
        .filter(|rp| rp.role_permission.resource.name.eq_ignore_ascii_case(resource_name))
        .map(|rp| rp.role_permission.permission.level)
        .max();

    match highest_level {
        Some(level) => permission_level_to_rustycog_permissions(level),
        None => vec![],
    }
}

/// Convert domain PermissionLevel to rustycog Permission levels
/// Returns all implied permissions (e.g., Owner implies Admin, Write, Read)
fn permission_level_to_rustycog_permissions(level: PermissionLevel) -> Vec<Permission> {
    match level {
        PermissionLevel::Owner => vec![Permission::Owner, Permission::Admin, Permission::Write, Permission::Read],
        PermissionLevel::Admin => vec![Permission::Admin, Permission::Write, Permission::Read],
        PermissionLevel::Write => vec![Permission::Write, Permission::Read],
        PermissionLevel::Read => vec![Permission::Read],
    }
}
