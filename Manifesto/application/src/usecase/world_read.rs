use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::{Project, ProjectMember},
    service::MemberService,
    value_objects::{OwnerType, PermissionLevel, ProjectStatus, Visibility},
};
use rustycog::core::error::DomainError;
use rustycog::permission::{Permission, PermissionChecker, ResourceRef, Subject};

use crate::ApplicationError;

pub const fn allows_world_read(project: &Project) -> bool {
    matches!(project.visibility, Visibility::Public)
        && matches!(project.status, ProjectStatus::Active)
}

fn permission_denied(message: &str) -> ApplicationError {
    ApplicationError::from(DomainError::permission_denied(message))
}

#[must_use]
pub fn member_is_project_admin(member: &ProjectMember) -> bool {
    member.is_project_owner() || member.has_permission("project", &PermissionLevel::Admin)
}

async fn is_org_admin(
    project: &Project,
    user_id: Uuid,
    org_permission_checker: &Arc<dyn PermissionChecker>,
) -> Result<bool, ApplicationError> {
    if project.owner_type != OwnerType::Organization {
        return Ok(false);
    }
    org_permission_checker
        .check(
            Subject::new(user_id),
            Permission::Admin,
            ResourceRef::new("organization", project.owner_id),
        )
        .await
        .map_err(ApplicationError::from)
}

async fn active_member(
    project_id: Uuid,
    user_id: Uuid,
    member_service: &Arc<dyn MemberService>,
) -> Result<Option<ProjectMember>, ApplicationError> {
    match member_service.get_member(project_id, user_id).await {
        Ok(member) if member.is_active() => Ok(Some(member)),
        Ok(_) => Ok(None),
        Err(DomainError::EntityNotFound { .. }) => Ok(None),
        Err(error) => Err(ApplicationError::from(error)),
    }
}

/// Fail-closed gate for project mutations: active membership or org admin,
/// and Suspended projects restricted to owner / project admin / org admin.
///
/// # Errors
///
/// Returns [`ApplicationError`] when the caller is not allowed to mutate.
pub async fn require_project_mutation_actor(
    project: &Project,
    user_id: Uuid,
    member_service: &Arc<dyn MemberService>,
    org_permission_checker: &Arc<dyn PermissionChecker>,
) -> Result<(), ApplicationError> {
    let org_admin = is_org_admin(project, user_id, org_permission_checker).await?;
    let member = active_member(project.id, user_id, member_service).await?;
    if project.status == ProjectStatus::Suspended {
        let allowed = member.as_ref().is_some_and(member_is_project_admin) || org_admin;
        if allowed {
            return Ok(());
        }
        return Err(permission_denied(
            "Suspended projects are only visible to owners and admins",
        ));
    }
    if member.is_some() || org_admin {
        return Ok(());
    }
    Err(permission_denied("Caller is not an active project member"))
}

/// Instance ACL: owner, generic component grant, exact UUID grant, org admin,
/// or world-readable public+active project.
///
/// # Errors
///
/// Returns [`ApplicationError`] on permission-checker failure.
pub async fn caller_can_read_component(
    project: &Project,
    component_id: Uuid,
    user_id: Option<Uuid>,
    member_service: &Arc<dyn MemberService>,
    org_permission_checker: &Arc<dyn PermissionChecker>,
) -> Result<bool, ApplicationError> {
    if allows_world_read(project) {
        return Ok(true);
    }
    let Some(uid) = user_id else {
        return Ok(false);
    };
    if let Some(member) = active_member(project.id, uid, member_service).await? {
        if member.is_project_owner()
            || member.has_permission("component", &PermissionLevel::Read)
            || member.has_permission(&component_id.to_string(), &PermissionLevel::Read)
            || member.has_permission(&format!("component:{component_id}"), &PermissionLevel::Read)
        {
            return Ok(true);
        }
    }
    is_org_admin(project, uid, org_permission_checker).await
}

/// Fail-closed world-read gate used after `OpenFGA` middleware.
///
/// # Errors
///
/// Returns [`ApplicationError`] when the project is not world-readable and the
/// caller is not a project member, an organization reader on an Internal
/// org-owned project, or an organization admin.
pub async fn enforce_world_read_or_principal(
    project: &Project,
    user_id: Option<Uuid>,
    member_service: &Arc<dyn MemberService>,
    org_permission_checker: &Arc<dyn PermissionChecker>,
) -> Result<(), ApplicationError> {
    if allows_world_read(project) {
        return Ok(());
    }
    let Some(uid) = user_id else {
        return Err(permission_denied(
            "Public-read is not available for this project",
        ));
    };
    if project.status == ProjectStatus::Suspended {
        if let Some(member) = active_member(project.id, uid, member_service).await? {
            if member_is_project_admin(&member) {
                return Ok(());
            }
        }
        if is_org_admin(project, uid, org_permission_checker).await? {
            return Ok(());
        }
        return Err(permission_denied(
            "Suspended projects are only visible to owners and admins",
        ));
    }
    if active_member(project.id, uid, member_service)
        .await?
        .is_some()
    {
        return Ok(());
    }
    if project.owner_type == OwnerType::Organization {
        let org_permission = if project.visibility == Visibility::Internal {
            Permission::Read
        } else {
            Permission::Admin
        };
        let allowed = org_permission_checker
            .check(
                Subject::new(uid),
                org_permission,
                ResourceRef::new("organization", project.owner_id),
            )
            .await
            .map_err(ApplicationError::from)?;
        if allowed {
            return Ok(());
        }
    }
    Err(permission_denied(
        "Public-read is not available for this project",
    ))
}
