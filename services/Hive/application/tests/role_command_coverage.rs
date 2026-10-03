use hive_application::{
    ApplicationError, CreateMemberRoleRequest, CreateRoleCommand, DeleteRoleCommand,
    GetRoleCommand, ListRolesCommand, PaginationRequest, RoleErrorMapper, UpdateMemberRoleRequest,
    UpdateRoleCommand,
};
use rustycog::command::{Command, CommandErrorMapper};
use rustycog::core::error::DomainError;
use uuid::Uuid;

#[test]
fn role_commands_validate_names_and_expose_types() {
    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let empty = CreateRoleCommand::new(
        org_id,
        &CreateMemberRoleRequest {
            name: "   ".to_string(),
            description: None,
            roles: vec![],
        },
        user_id,
    );
    assert!(empty.validate().is_err());
    let created = CreateRoleCommand::new(
        org_id,
        &CreateMemberRoleRequest {
            name: "reader".to_string(),
            description: Some("r".to_string()),
            roles: vec![],
        },
        user_id,
    );
    assert_eq!(created.command_type(), "create_role");
    assert!(created.validate().is_ok());
    assert_eq!(created.command_id(), created.command_id);

    let listed = ListRolesCommand::new(org_id, PaginationRequest::default());
    assert_eq!(listed.command_type(), "list_roles");
    assert!(listed.validate().is_ok());

    let got = GetRoleCommand::new(org_id, Uuid::new_v4());
    assert_eq!(got.command_type(), "get_role");
    assert!(got.validate().is_ok());

    let empty_update = UpdateRoleCommand::new(
        org_id,
        Uuid::new_v4(),
        &UpdateMemberRoleRequest {
            name: Some(" ".to_string()),
            description: None,
            roles: None,
        },
        user_id,
    );
    assert!(empty_update.validate().is_err());
    let update = UpdateRoleCommand::new(
        org_id,
        Uuid::new_v4(),
        &UpdateMemberRoleRequest {
            name: Some("writer".to_string()),
            description: None,
            roles: None,
        },
        user_id,
    );
    assert_eq!(update.command_type(), "update_role");
    assert!(update.validate().is_ok());

    let delete = DeleteRoleCommand::new(org_id, Uuid::new_v4(), user_id);
    assert_eq!(delete.command_type(), "delete_role");
    assert!(delete.validate().is_ok());
}

#[test]
fn role_error_mapper_covers_application_error_variants() {
    let mapper = RoleErrorMapper;
    let domain_error = mapper.map_error(Box::new(ApplicationError::Domain(
        DomainError::invalid_input("bad"),
    )));
    assert!(
        domain_error.to_string().contains("bad") || domain_error.to_string().contains("domain")
    );
    let _ = mapper.map_error(Box::new(ApplicationError::validation_error(vec![])));
    let _ = mapper.map_error(Box::new(ApplicationError::external_service_error(
        "github", "down",
    )));
    let _ = mapper.map_error(Box::new(ApplicationError::rate_limit("slow")));
    let _ = mapper.map_error(Box::new(ApplicationError::internal_error("boom")));
    let _ = mapper.map_error(Box::new(std::io::Error::other("unknown")));
}
