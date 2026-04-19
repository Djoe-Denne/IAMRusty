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
        user_id: Option<Uuid>,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        let mut permissions = vec![];
        let Some((project_resource, remaining_resources)) = resource_ids.split_first() else {
            return Ok(permissions);
        };

        if !remaining_resources.is_empty() {
            debug!(
                "ProjectPermissionFetcher ignoring {} additional resource IDs",
                remaining_resources.len()
            );
        }

        let project_id = project_resource.id();
        debug!("Fetching permissions for user {:?} on project {}", user_id, project_id);

        // Get the project to check visibility
        let project = self.project_service.get_project(&project_id).await?;

        if project.is_public() {
            permissions.push(Permission::Read);
        }

        // Get the member's role on the project
        let Some(user_id) = user_id else {
            return Ok(permissions);
        };

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
/// 
/// Supports both generic component permissions (applies to all components) and
/// specific component permissions (identified by component UUID).
/// 
/// When checking permissions:
/// - resource_ids[0] = project_id
/// - resource_ids[1] = component_id (optional)
/// 
/// Returns the highest permission level between generic and specific permissions.
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
        user_id: Option<Uuid>,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        let mut permissions = vec![];
        // Component permissions are based on project membership
        // resource_ids[0] should be the project_id
        // resource_ids[1] should be the component_id (optional)
        let Some((project_resource, remaining_resources)) = resource_ids.split_first() else {
            return Ok(permissions);
        };
        let project_id = project_resource.id();
        let component_id = remaining_resources.first().map(|resource| resource.id());

        if remaining_resources.len() > 1 {
            debug!(
                "ComponentPermissionFetcher ignoring {} additional resource IDs beyond component scope",
                remaining_resources.len() - 1
            );
        }
        
        debug!(
            "Fetching component permissions for user {:?} on project {}, component {:?}",
            user_id, project_id, component_id
        );

        let project = self.project_service.get_project(&project_id).await?;

        if project.is_public() {
            permissions.push(Permission::Read);
        }

        let Some(user_id) = user_id else {
            return Ok(permissions);
        };

        let member = match self.member_service.get_member(project_id, user_id).await {
            Ok(member) => member,
            Err(_) => return Ok(permissions),
        };

        if !member.is_active() {
            return Ok(permissions);
        }

        // Get generic "component" permissions (applies to all components)
        let generic_permissions = get_permissions_for_resource(&member.role_permissions, "component");
        
        // Get specific component permissions if component_id is provided
        // Note: Component instance resources use just the UUID as the resource ID
        let specific_permissions = if let Some(comp_id) = component_id {
            let component_resource_id = comp_id.to_string();
            get_permissions_for_resource(&member.role_permissions, &component_resource_id)
        } else {
            vec![]
        };

        // Combine permissions - take the highest level from either source
        let combined = combine_highest_permissions(generic_permissions, specific_permissions);
        permissions.extend(combined);

        debug!(
            "User {} has permissions {:?} on project {} component {:?}", 
            user_id, permissions, project_id, component_id
        );

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
        user_id: Option<Uuid>,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        let mut permissions = vec![];
        // Member management permissions are based on project membership
        let Some((project_resource, remaining_resources)) = resource_ids.split_first() else {
            return Ok(permissions);
        };

        if !remaining_resources.is_empty() {
            debug!(
                "MemberPermissionFetcher ignoring {} additional resource IDs",
                remaining_resources.len()
            );
        }

        let project_id = project_resource.id();
        debug!("Fetching member permissions for user {:?} on project {}", user_id, project_id);

        let Some(user_id) = user_id else {
            return Ok(permissions);
        };

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

/// Combine permissions from multiple sources, returning the highest permission level
/// This allows users to get the best of generic and specific permissions
fn combine_highest_permissions(
    generic: Vec<Permission>,
    specific: Vec<Permission>,
) -> Vec<Permission> {
    // Determine the highest permission level from both sources
    let generic_level = highest_permission_level(&generic);
    let specific_level = highest_permission_level(&specific);

    // Return whichever has the higher level, or generic if equal/both None
    match (generic_level, specific_level) {
        (Some(g), Some(s)) if s > g => specific,
        (None, Some(_)) => specific,
        _ => generic,
    }
}

/// Get the highest permission level from a list of permissions
fn highest_permission_level(permissions: &[Permission]) -> Option<u8> {
    permissions.iter().map(|p| permission_to_level(p)).max()
}

/// Convert a Permission to a numeric level for comparison
fn permission_to_level(permission: &Permission) -> u8 {
    match permission {
        Permission::Read => 1,
        Permission::Write => 2,
        Permission::Admin => 3,
        Permission::Owner => 4,
    }
}
