//! QA coverage90-local B: pure permission hierarchy and membership revocation.
use chrono::{Duration, TimeZone, Utc};
use manifesto_domain::{
    DomainError, MemberSource, Permission, PermissionLevel, ProjectMember,
    ProjectMemberRolePermission, Resource, RolePermission,
};
use uuid::Uuid;

fn member() -> ProjectMember {
    ProjectMember::new(Uuid::new_v4(), Uuid::new_v4(), MemberSource::Direct, None)
}

fn grant(member: &mut ProjectMember, resource: &str, level: PermissionLevel) {
    let role = RolePermission::new(
        None,
        None,
        member.project_id,
        Permission::from_level(level),
        Resource::from(resource),
        None,
    );
    let assignment = ProjectMemberRolePermission::new(None, member.id, role, Utc::now());
    member.role_permissions.push(assignment);
}

#[test]
fn permission_hierarchy_is_scoped_to_resource_and_case_insensitive() {
    use PermissionLevel::{Admin, Owner, Read, Write};
    for (level, expected) in [
        (Read, [true, false, false, false]),
        (Write, [true, true, false, false]),
        (Admin, [true, true, true, false]),
        (Owner, [true, true, true, true]),
    ] {
        let mut member = member();
        grant(&mut member, "Project", level);
        for (required, expected) in [Read, Write, Admin, Owner].into_iter().zip(expected) {
            assert_eq!(
                member.has_permission("pRoJeCt", &required),
                expected,
                "{level:?} requiring {required:?}"
            );
            assert!(!member.has_permission("component", &required));
        }
        assert_eq!(member.get_permission_for_resource("PROJECT"), Some(level));
        assert_eq!(member.get_permission_for_resource("component"), None);
    }
}

#[test]
fn removal_revokes_permissions_ownership_and_permission_edits() {
    let mut member = member();
    member.is_owner = true;
    grant(&mut member, "project", PermissionLevel::Owner);
    assert!(member.is_project_owner());
    let assignments = member.role_permissions.clone();
    member.remove(Some("Left organization".to_owned()), None);
    assert!(!member.is_active());
    assert!(!member.is_owner);
    assert!(!member.is_project_owner());
    assert!(!member.has_permission("project", &PermissionLevel::Read));
    assert_eq!(member.get_permission_for_resource("project"), None);
    assert!(matches!(
        member.update_role_permissions(Vec::new()),
        Err(DomainError::BusinessRuleViolation { .. })
    ));
    assert_eq!(member.role_permissions.len(), assignments.len());
    assert_eq!(
        member.role_permissions[0].role_permission.permission.level,
        PermissionLevel::Owner
    );
    assert_eq!(member.removal_reason.as_deref(), Some("Left organization"));
    assert!(member.grace_period_ends_at.is_none());
}

#[test]
fn active_permission_replacement_revokes_previous_resource_grants() {
    let mut member = member();
    grant(&mut member, "project", PermissionLevel::Admin);
    let mut replacement = member.clone();
    replacement.role_permissions.clear();
    grant(&mut replacement, "component", PermissionLevel::Read);
    member
        .update_role_permissions(replacement.role_permissions)
        .unwrap();
    assert!(!member.has_permission("project", &PermissionLevel::Read));
    assert!(member.has_permission("component", &PermissionLevel::Read));
    assert!(!member.has_permission("component", &PermissionLevel::Write));
}

#[test]
fn management_requires_strictly_higher_rank_and_never_targets_owner() {
    use PermissionLevel::{Admin, Owner, Read, Write};
    for (manager_level, expected) in [
        (Read, [false, false, false, false]),
        (Write, [true, false, false, false]),
        (Admin, [true, true, false, false]),
        (Owner, [true, true, true, false]),
    ] {
        let mut manager = member();
        grant(&mut manager, "project", manager_level);
        for (target_level, expected) in [Read, Write, Admin, Owner].into_iter().zip(expected) {
            let mut target = member();
            target.project_id = manager.project_id;
            grant(&mut target, "project", target_level);
            assert_eq!(
                manager.can_manage_member(&target),
                expected,
                "manager={manager_level:?}, target={target_level:?}"
            );
        }
    }
}

#[test]
fn management_fallback_and_removed_members_fail_closed() {
    let mut manager = member();
    let mut target = member();
    target.project_id = manager.project_id;
    assert!(!manager.can_manage_member(&target));
    grant(&mut manager, "member", PermissionLevel::Admin);
    assert!(manager.can_manage_member(&target));
    target.is_owner = true;
    assert!(!manager.can_manage_member(&target));
    target.is_owner = false;
    target.remove(None, None);
    assert!(!manager.can_manage_member(&target));
    let mut active_target = member();
    active_target.project_id = manager.project_id;
    manager.remove(None, None);
    assert!(!manager.can_manage_member(&active_target));
}

#[test]
fn project_resource_takes_precedence_over_member_management_fallback() {
    let mut manager = member();
    grant(&mut manager, "member", PermissionLevel::Owner);
    grant(&mut manager, "project", PermissionLevel::Read);
    let mut target = member();
    target.project_id = manager.project_id;
    assert!(
        !manager.can_manage_member(&target),
        "member grant must not override explicit project rank"
    );
}

#[test]
fn grace_boundary_is_exclusive_and_missing_deadline_cannot_restore() {
    let now = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
    let mut member = member();
    assert!(!member.is_within_grace(now));
    assert!(matches!(
        member.restore(now),
        Err(DomainError::BusinessRuleViolation { .. })
    ));
    member.remove(Some("Removed".to_owned()), None);
    assert!(!member.is_within_grace(now));
    assert!(matches!(
        member.restore(now),
        Err(DomainError::BusinessRuleViolation { .. })
    ));
    member.removed_at = Some(now - Duration::days(1));
    member.grace_period_ends_at = Some(now);
    assert!(member.is_within_grace(now - Duration::nanoseconds(1)));
    assert!(!member.is_within_grace(now));
    assert!(matches!(
        member.restore(now),
        Err(DomainError::BusinessRuleViolation { .. })
    ));
    assert!(member.removed_at.is_some());
    assert_eq!(member.removal_reason.as_deref(), Some("Removed"));
}

#[test]
fn restore_clears_removal_reason_and_grace_deadline() {
    let mut member = member();
    member.remove(Some("Temporary".to_owned()), Some(7));
    assert!(member.grace_period_ends_at.is_some());
    member.restore(Utc::now()).unwrap();
    assert_eq!(member.removal_reason, None);
    assert_eq!(member.grace_period_ends_at, None);
}

#[test]
fn removal_reason_validation_accepts_limit_and_rejects_overflow() {
    let mut member = member();
    member.removal_reason = Some("r".repeat(100));
    assert!(member.validate().is_ok());
    member.removal_reason = Some("r".repeat(101));
    assert!(matches!(
        member.validate(),
        Err(DomainError::InvalidInput { .. })
    ));
}
