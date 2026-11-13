//! QA coverage90-local B: validation codes for malformed project/member commands.
use manifesto_application::command::member::{
    AddMemberCommand, GrantPermissionCommand, RevokePermissionCommand, UpdateMemberCommand,
};
use manifesto_application::command::project::{CreateProjectCommand, UpdateProjectCommand};
use manifesto_application::dto::{
    AddMemberRequest, CreateProjectRequest, GrantPermissionRequest, ResourcePermissionRequest,
    UpdateMemberPermissionsRequest, UpdateProjectRequest,
};
use manifesto_domain::FieldUpdate;
use rustycog::command::{Command, CommandError};
use uuid::Uuid;

fn assert_validation(result: Result<(), CommandError>, expected: &str) {
    let error = result.expect_err("malformed command must fail validation");
    assert!(
        matches!(error, CommandError::Validation { .. }),
        "{error:?}"
    );
    assert_eq!(error.code(), expected);
}

#[test]
fn project_creation_and_rename_reject_empty_or_whitespace_only_names() {
    for name in ["", " ", "\t\n"] {
        let request = CreateProjectRequest {
            name: name.to_owned(),
            description: None,
            owner_type: "personal".to_owned(),
            owner_id: None,
            visibility: None,
        };
        assert_validation(
            CreateProjectCommand::new(request, Uuid::new_v4()).validate(),
            "empty_name",
        );
        let request = UpdateProjectRequest {
            name: Some(name.to_owned()),
            description: FieldUpdate::Unchanged,
            visibility: None,
        };
        assert_validation(
            UpdateProjectCommand::new(Uuid::new_v4(), request, Uuid::new_v4()).validate(),
            "empty_name",
        );
    }
}

#[test]
fn project_patch_can_omit_name_and_clear_description_without_being_a_rename() {
    for name in [None, Some("  Hive catalog  ".to_owned())] {
        let request = UpdateProjectRequest {
            name,
            description: FieldUpdate::Set(None),
            visibility: Some("private".to_owned()),
        };
        assert!(
            UpdateProjectCommand::new(Uuid::new_v4(), request, Uuid::new_v4())
                .validate()
                .is_ok()
        );
    }
}

#[test]
fn adding_member_rejects_blank_and_unknown_permission_levels() {
    for (permission, code) in [
        ("", "empty_permission"),
        (" \t", "empty_permission"),
        ("delete", "invalid_permission"),
        ("superuser", "invalid_permission"),
    ] {
        let request = AddMemberRequest {
            user_id: Uuid::new_v4(),
            resource: None,
            permission: permission.to_owned(),
        };
        assert_validation(
            AddMemberCommand::new(Uuid::new_v4(), request, Uuid::new_v4()).validate(),
            code,
        );
    }
}

#[test]
fn adding_member_accepts_each_documented_permission_case_insensitively() {
    for permission in ["read", "WRITE", "Admin", "owner"] {
        let request = AddMemberRequest {
            user_id: Uuid::new_v4(),
            resource: None,
            permission: permission.to_owned(),
        };
        assert!(
            AddMemberCommand::new(Uuid::new_v4(), request, Uuid::new_v4())
                .validate()
                .is_ok(),
            "{permission}"
        );
    }
}

#[test]
fn member_permission_update_requires_at_least_one_valid_permission() {
    let request = UpdateMemberPermissionsRequest {
        permissions: Vec::new(),
    };
    assert_validation(
        UpdateMemberCommand::new(Uuid::new_v4(), Uuid::new_v4(), request, Uuid::new_v4())
            .validate(),
        "empty_permissions",
    );
    for (resource, permission, code) in [
        (" \t", "read", "empty_resource"),
        ("project", "\n", "empty_permission"),
        ("project", "execute", "invalid_permission"),
    ] {
        // A valid first entry must not hide an invalid subsequent entry.
        let request = UpdateMemberPermissionsRequest {
            permissions: vec![
                ResourcePermissionRequest {
                    resource: "member".to_owned(),
                    permission: "read".to_owned(),
                },
                ResourcePermissionRequest {
                    resource: resource.to_owned(),
                    permission: permission.to_owned(),
                },
            ],
        };
        assert_validation(
            UpdateMemberCommand::new(Uuid::new_v4(), Uuid::new_v4(), request, Uuid::new_v4())
                .validate(),
            code,
        );
    }
}

#[test]
fn member_permission_update_accepts_a_multi_resource_permission_set() {
    let request = UpdateMemberPermissionsRequest {
        permissions: vec![
            ResourcePermissionRequest {
                resource: "project".to_owned(),
                permission: "ADMIN".to_owned(),
            },
            ResourcePermissionRequest {
                resource: "member".to_owned(),
                permission: "Write".to_owned(),
            },
        ],
    };
    assert!(
        UpdateMemberCommand::new(Uuid::new_v4(), Uuid::new_v4(), request, Uuid::new_v4())
            .validate()
            .is_ok()
    );
}

#[test]
fn grant_validates_resource_and_permission_independently() {
    for (resource, permission, code) in [
        ("", "read", "empty_resource"),
        ("\n", "read", "empty_resource"),
        ("project", "", "empty_permission"),
        ("project", " \t", "empty_permission"),
        ("project", "delete", "invalid_permission"),
    ] {
        let request = GrantPermissionRequest {
            resource: resource.to_owned(),
            permission: permission.to_owned(),
        };
        assert_validation(
            GrantPermissionCommand::new(Uuid::new_v4(), Uuid::new_v4(), request, Uuid::new_v4())
                .validate(),
            code,
        );
    }
    for permission in ["read", "WRITE", "Admin", "owner"] {
        let request = GrantPermissionRequest {
            resource: "project".to_owned(),
            permission: permission.to_owned(),
        };
        assert!(
            GrantPermissionCommand::new(Uuid::new_v4(), Uuid::new_v4(), request, Uuid::new_v4())
                .validate()
                .is_ok(),
            "{permission}"
        );
    }
}

#[test]
fn revoke_requires_a_resource_and_accepts_a_named_resource() {
    for resource in ["", " \t\n"] {
        assert_validation(
            RevokePermissionCommand::new(
                Uuid::new_v4(),
                Uuid::new_v4(),
                resource.to_owned(),
                Uuid::new_v4(),
            )
            .validate(),
            "empty_resource",
        );
    }
    assert!(RevokePermissionCommand::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        "project".to_owned(),
        Uuid::new_v4()
    )
    .validate()
    .is_ok());
}
