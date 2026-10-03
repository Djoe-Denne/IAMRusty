use chrono::Utc;
use manifesto_domain::value_objects::PermissionLevel;
use manifesto_domain::{
    Permission, PermissionResourceCombo, ProjectMemberRolePermission, Resource, RolePermission,
};
use uuid::Uuid;

#[test]
fn manifesto_permission_and_resource_entities() {
    let permission = Permission::new(PermissionLevel::Write, Some(Utc::now()));
    let from_level = Permission::from_level(PermissionLevel::Admin);
    let converted = Permission::from(PermissionLevel::Read);
    assert_eq!(permission.level, PermissionLevel::Write);
    assert_eq!(from_level.level, PermissionLevel::Admin);
    assert_eq!(converted.level, PermissionLevel::Read);

    let named = Resource::new("project".to_string(), None);
    let from_string = Resource::from("component".to_string());
    let from_str = Resource::from("member");
    assert_eq!(named.name, "project");
    assert_eq!(from_string.name, "component");
    assert_eq!(from_str.name, "member");
}

#[test]
fn manifesto_role_and_member_role_permissions() {
    let project_id = Uuid::new_v4();
    let mut role = RolePermission::new(
        Some(Uuid::new_v4()),
        None,
        project_id,
        Permission::from_level(PermissionLevel::Read),
        Resource::from("project"),
        Some(Utc::now()),
    );
    role.update_name("reader".to_string());
    assert_eq!(role.name.as_deref(), Some("reader"));

    let combo = PermissionResourceCombo::new("admin".to_string(), "project".to_string());
    assert_eq!(combo.permission_level, "admin");

    let assignment =
        ProjectMemberRolePermission::new(Some(Uuid::new_v4()), Uuid::new_v4(), role, Utc::now());
    assert_eq!(assignment.role_permission.project_id, project_id);
}
