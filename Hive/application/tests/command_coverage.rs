use hive_application::{
    AddMemberCommand, AddMemberRequest, ApplicationError, CancelInvitationCommand,
    CreateExternalLinkCommand, CreateExternalLinkRequest, CreateInvitationCommand,
    CreateInvitationRequest, CreateOrganizationCommand, CreateOrganizationRequest,
    DeleteOrganizationCommand, ExternalLinkErrorMapper, GetMemberCommand, GetOrganizationCommand,
    InvitationErrorMapper, ListMembersCommand, ListOrganizationsCommand, MemberErrorMapper,
    MemberRole, MemberRolePermission, OrganizationErrorMapper, PaginationRequest,
    RemoveMemberCommand, SearchOrganizationsCommand, StartSyncJobCommand, StartSyncJobRequest,
    SyncJobErrorMapper, UpdateMemberCommand, UpdateMemberRolesRequest, UpdateOrganizationCommand,
    UpdateOrganizationRequest, ValidationError,
};
use rustycog::command::{Command, CommandErrorMapper};
use rustycog::core::error::DomainError;
use uuid::Uuid;

fn application_errors() -> Vec<ApplicationError> {
    vec![
        ApplicationError::Domain(DomainError::invalid_input("bad")),
        ApplicationError::validation_error(vec![ValidationError::simple("name", "empty")]),
        ApplicationError::external_service_error("provider", "down"),
        ApplicationError::rate_limit("slow down"),
        ApplicationError::internal_error("boom"),
        ApplicationError::single_validation_error("slug", "empty"),
    ]
}

fn map_all(mapper: &impl CommandErrorMapper) {
    for error in application_errors() {
        let _ = mapper.map_error(Box::new(error));
    }
    let _ = mapper.map_error(Box::new(std::io::Error::other("plain")));
}

#[test]
fn organization_commands_validate_and_map_errors() {
    let user_id = Uuid::new_v4();
    let empty = CreateOrganizationCommand::new(
        CreateOrganizationRequest {
            name: " ".to_string(),
            slug: "hive".to_string(),
            description: None,
            avatar_url: None,
        },
        user_id,
    );
    assert!(empty.validate().is_err());
    let empty_slug = CreateOrganizationCommand::new(
        CreateOrganizationRequest {
            name: "Hive".to_string(),
            slug: " ".to_string(),
            description: None,
            avatar_url: None,
        },
        user_id,
    );
    assert!(empty_slug.validate().is_err());
    let created = CreateOrganizationCommand::new(
        CreateOrganizationRequest {
            name: "Hive".to_string(),
            slug: "hive".to_string(),
            description: None,
            avatar_url: None,
        },
        user_id,
    );
    assert_eq!(created.command_type(), "create_organization");
    assert!(created.validate().is_ok());

    let org_id = Uuid::new_v4();
    assert_eq!(
        GetOrganizationCommand::new(org_id, Some(user_id)).command_type(),
        "get_organization"
    );
    assert_eq!(
        DeleteOrganizationCommand::new(org_id, user_id).command_type(),
        "delete_organization"
    );
    assert_eq!(
        ListOrganizationsCommand::new(user_id, PaginationRequest::default()).command_type(),
        "list_organizations"
    );
    let empty_update = UpdateOrganizationCommand::new(
        org_id,
        UpdateOrganizationRequest {
            name: Some(" ".to_string()),
            description: None,
            avatar_url: None,
            settings: None,
        },
        user_id,
    );
    assert!(empty_update.validate().is_err());
    let update = UpdateOrganizationCommand::new(
        org_id,
        UpdateOrganizationRequest {
            name: Some("Hive".to_string()),
            description: None,
            avatar_url: None,
            settings: None,
        },
        user_id,
    );
    assert_eq!(update.command_type(), "update_organization");
    assert!(update.validate().is_ok());
    assert_eq!(
        SearchOrganizationsCommand::new(
            hive_application::OrganizationSearchRequest::default(),
            Some(user_id)
        )
        .command_type(),
        "search_organizations"
    );
    map_all(&OrganizationErrorMapper);
}

#[test]
fn member_invitation_sync_and_link_commands_cover_types_and_mappers() {
    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let request = AddMemberRequest {
        user_id: Uuid::new_v4(),
        roles: vec![MemberRole {
            organization_id: org_id,
            resource: "organization".to_string(),
            permissions: MemberRolePermission::Read,
        }],
    };
    let add = AddMemberCommand::new(org_id, &request, user_id);
    assert_eq!(add.command_type(), "add_member");
    assert!(add.validate().is_ok());
    assert_eq!(
        RemoveMemberCommand::new(org_id, request.user_id, user_id).command_type(),
        "remove_member"
    );
    assert_eq!(
        ListMembersCommand::new(org_id, PaginationRequest::default(), Some(user_id)).command_type(),
        "list_members"
    );
    assert_eq!(
        GetMemberCommand::new(org_id, request.user_id, Some(user_id)).command_type(),
        "get_member"
    );
    let update = UpdateMemberCommand::new(
        org_id,
        request.user_id,
        &UpdateMemberRolesRequest {
            roles: request.roles.clone(),
        },
        user_id,
    );
    assert_eq!(update.command_type(), "update_member");
    map_all(&MemberErrorMapper);

    let invitation = CreateInvitationCommand::new(
        org_id,
        CreateInvitationRequest {
            email: "ada@example.test".to_string(),
            roles: request.roles,
            message: Some("welcome".to_string()),
        },
        user_id,
    );
    assert_eq!(invitation.command_type(), "create_invitation");
    assert_eq!(
        CancelInvitationCommand::new(org_id, Uuid::new_v4(), Some(user_id)).command_type(),
        "cancel_invitation"
    );
    map_all(&InvitationErrorMapper);

    let sync = StartSyncJobCommand::new(
        org_id,
        StartSyncJobRequest {
            external_link_id: Uuid::new_v4(),
            job_type: "full_sync".to_string(),
            options: None,
        },
        user_id,
    );
    assert_eq!(sync.command_type(), "start_sync_job");
    map_all(&SyncJobErrorMapper);

    let link_request = CreateExternalLinkRequest {
        provider_id: Uuid::new_v4(),
        provider_config: serde_json::json!({"org": "acme"}),
        sync_enabled: Some(true),
        sync_settings: None,
    };
    let link = CreateExternalLinkCommand::new(org_id, &link_request, user_id);
    assert_eq!(link.command_type(), "create_external_link");
    map_all(&ExternalLinkErrorMapper);
}
