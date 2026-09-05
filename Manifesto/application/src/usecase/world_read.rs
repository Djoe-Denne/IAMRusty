use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::Project,
    service::MemberService,
    value_objects::{OwnerType, ProjectStatus, Visibility},
};
use rustycog::core::error::DomainError;
use rustycog::permission::{Permission, PermissionChecker, ResourceRef, Subject};

use crate::ApplicationError;

pub(crate) fn allows_world_read(project: &Project) -> bool {
    matches!(project.visibility, Visibility::Public)
        && matches!(project.status, ProjectStatus::Draft | ProjectStatus::Active)
}

fn permission_denied(message: &str) -> ApplicationError {
    ApplicationError::from(DomainError::permission_denied(message))
}

/// Fail-closed world-read gate used after OpenFGA middleware.
///
/// # Errors
///
/// Returns [`ApplicationError`] when the project is not world-readable and the
/// caller is not the owner, a project member, an organization reader on an
/// Internal org-owned project, or an organization admin on a private one.
pub(crate) async fn enforce_world_read_or_principal(
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
    if project.created_by == uid
        || (project.owner_type == OwnerType::Personal && project.owner_id == uid)
    {
        return Ok(());
    }
    if member_service
        .check_member_exists(&project.id, &uid)
        .await?
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
