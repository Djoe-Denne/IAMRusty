//! QA coverage90-local B: membership state machine, including rejected transitions.
use chrono::{TimeZone, Utc};
use hive_domain::{
    MemberStatus, OrganizationMember, OrganizationMemberRolePermission, Permission,
    PermissionLevel, Resource, RolePermission,
};
use rustycog::core::error::DomainError;
use uuid::Uuid;

fn member(status: MemberStatus) -> OrganizationMember {
    let mut member = OrganizationMember::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        "https://identity.example.test/hive",
        None,
    );
    member.id = Some(Uuid::new_v4());
    member.status = status;
    member.updated_at = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
    if member.is_pending() {
        member.joined_at = None;
    }
    member
}

fn assignment(
    member: &OrganizationMember,
    level: PermissionLevel,
) -> OrganizationMemberRolePermission {
    let permission = Permission::new(level, None, None);
    let role = RolePermission::new(
        None,
        None,
        member.organization_id,
        &permission,
        &Resource::from("organization".to_owned()),
        None,
    );
    OrganizationMemberRolePermission::new(
        None,
        &member.organization_id,
        &member.id.expect("fixture member ID"),
        &role,
        Utc::now(),
    )
}

#[test]
fn pending_invitee_activation_records_join_and_preserves_principal() {
    let mut member = member(MemberStatus::Pending);
    let before = member.clone();
    member.activate().unwrap();
    assert!(member.is_active());
    assert!(!member.is_pending());
    assert!(!member.is_suspended());
    assert!(member
        .joined_at
        .is_some_and(|joined| joined > before.updated_at));
    assert!(member.updated_at > before.updated_at);
    assert_eq!(member.user_id, before.user_id);
    assert_eq!(member.issuer, before.issuer);
    assert_eq!(member.organization_id, before.organization_id);
}

#[test]
fn suspension_then_reactivation_preserves_original_join_and_roles() {
    let mut member = member(MemberStatus::Active);
    let role = assignment(&member, PermissionLevel::Read);
    member.roles.push(role);
    let before = member.clone();
    member.suspend().unwrap();
    assert!(member.is_suspended());
    assert!(!member.is_active());
    assert_eq!(member.joined_at, before.joined_at);
    member.reactivate().unwrap();
    assert!(member.is_active());
    assert!(!member.is_suspended());
    assert_eq!(member.joined_at, before.joined_at);
    assert_eq!(member.roles, before.roles);
    assert!(member.updated_at > before.updated_at);
}

#[test]
fn replacing_membership_roles_removes_previous_grants_and_allows_clearing() {
    let mut member = member(MemberStatus::Active);
    let role = assignment(&member, PermissionLevel::Admin);
    member.roles.push(role);
    let replacement = assignment(&member, PermissionLevel::Read);
    member.update_roles(vec![replacement.clone()]);
    assert_eq!(member.roles, vec![replacement]);
    member.update_roles(Vec::new());
    assert!(
        member.roles.is_empty(),
        "cleared grants must not remain attached"
    );
}

#[test]
fn invalid_membership_transitions_leave_entire_membership_unchanged() {
    // Mutation caught: allowing duplicate activation, suspending an invitee, or
    // using activate to bypass suspension instead of explicit reactivation.
    for status in [
        MemberStatus::Pending,
        MemberStatus::Active,
        MemberStatus::Suspended,
    ] {
        for action in 0..3 {
            let allowed = matches!(
                (&status, action),
                (MemberStatus::Pending, 0)
                    | (MemberStatus::Active, 1)
                    | (MemberStatus::Suspended, 2)
            );
            if allowed {
                continue;
            }
            let mut member = member(status.clone());
            let before = member.clone();
            let result = match action {
                0 => member.activate(),
                1 => member.suspend(),
                _ => member.reactivate(),
            };
            assert!(
                matches!(result, Err(DomainError::BusinessRuleViolation { .. })),
                "status={status:?}, action={action}, result={result:?}"
            );
            assert_eq!(
                member, before,
                "rejected action {action} mutated {status:?}"
            );
        }
    }
}
