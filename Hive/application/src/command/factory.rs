use super::{
    external_link::{
        CreateExternalLinkCommand, CreateExternalLinkCommandHandler, ExternalLinkErrorMapper,
    },
    invitation::{
        AcceptInvitationCommand, AcceptInvitationCommandHandler, CancelInvitationCommand,
        CancelInvitationCommandHandler, CreateInvitationCommand, CreateInvitationCommandHandler,
        InvitationErrorMapper,
    },
    member::{
        AddMemberCommand, AddMemberCommandHandler, GetMemberCommand, GetMemberCommandHandler,
        ListMembersCommand, ListMembersCommandHandler, MemberErrorMapper, RemoveMemberCommand,
        RemoveMemberCommandHandler, UpdateMemberCommand, UpdateMemberCommandHandler,
    },
    organization::{
        CreateOrganizationCommand, CreateOrganizationCommandHandler, DeleteOrganizationCommand,
        DeleteOrganizationCommandHandler, GetOrganizationCommand, GetOrganizationCommandHandler,
        ListOrganizationsCommand, ListOrganizationsCommandHandler, OrganizationErrorMapper,
        SearchOrganizationsCommand, SearchOrganizationsCommandHandler, UpdateOrganizationCommand,
        UpdateOrganizationCommandHandler,
    },
    role::{
        GetRoleCommand, GetRoleCommandHandler, ListRolesCommand, ListRolesCommandHandler,
        RoleErrorMapper,
    },
    sync_job::{StartSyncJobCommand, StartSyncJobCommandHandler, SyncJobErrorMapper},
};
use crate::usecase::{
    ExternalLinkUseCase, InvitationUseCase, MemberUseCase, OrganizationUseCase, RoleUseCase,
    SyncJobUseCase,
};
use rustycog::command::{CommandRegistry, CommandRegistryBuilder, RegistryConfig};
use rustycog::config::CommandConfig;
use std::sync::Arc;

/// Factory for creating a command registry with all Hive commands registered
pub struct HiveCommandRegistryFactory;

impl HiveCommandRegistryFactory {
    /// Create a command registry with all Hive commands registered
    pub fn create_hive_registry(
        organization_usecase: Arc<dyn OrganizationUseCase>,
        member_usecase: Arc<dyn MemberUseCase>,
        invitation_usecase: Arc<dyn InvitationUseCase>,
        external_link_usecase: Arc<dyn ExternalLinkUseCase>,
        sync_job_usecase: Arc<dyn SyncJobUseCase>,
        role_usecase: Arc<dyn RoleUseCase>,
        command_config: &CommandConfig,
    ) -> CommandRegistry {
        let builder = CommandRegistryBuilder::with_config(RegistryConfig::from_retry_config(
            &command_config.retry,
        ));
        let builder = register_organization_commands(builder, organization_usecase);
        let builder = register_member_commands(builder, member_usecase);
        let builder = register_role_commands(builder, role_usecase);
        let builder = register_invitation_commands(builder, invitation_usecase);
        let builder = register_external_link_commands(builder, external_link_usecase);
        let builder = register_sync_job_commands(builder, sync_job_usecase);
        builder.build()
    }
}

fn register_organization_commands(
    builder: CommandRegistryBuilder,
    organization_usecase: Arc<dyn OrganizationUseCase>,
) -> CommandRegistryBuilder {
    let create_org_handler = Arc::new(CreateOrganizationCommandHandler::new(
        organization_usecase.clone(),
    ));
    let get_org_handler = Arc::new(GetOrganizationCommandHandler::new(
        organization_usecase.clone(),
    ));
    let update_org_handler = Arc::new(UpdateOrganizationCommandHandler::new(
        organization_usecase.clone(),
    ));
    let delete_org_handler = Arc::new(DeleteOrganizationCommandHandler::new(
        organization_usecase.clone(),
    ));
    let list_org_handler = Arc::new(ListOrganizationsCommandHandler::new(
        organization_usecase.clone(),
    ));
    let search_org_handler = Arc::new(SearchOrganizationsCommandHandler::new(organization_usecase));
    let org_error_mapper = Arc::new(OrganizationErrorMapper);

    builder
        .register::<CreateOrganizationCommand, _>(
            "create_organization".to_string(),
            create_org_handler,
            org_error_mapper.clone(),
        )
        .register::<GetOrganizationCommand, _>(
            "get_organization".to_string(),
            get_org_handler,
            org_error_mapper.clone(),
        )
        .register::<UpdateOrganizationCommand, _>(
            "update_organization".to_string(),
            update_org_handler,
            org_error_mapper.clone(),
        )
        .register::<DeleteOrganizationCommand, _>(
            "delete_organization".to_string(),
            delete_org_handler,
            org_error_mapper.clone(),
        )
        .register::<ListOrganizationsCommand, _>(
            "list_organizations".to_string(),
            list_org_handler,
            org_error_mapper.clone(),
        )
        .register::<SearchOrganizationsCommand, _>(
            "search_organizations".to_string(),
            search_org_handler,
            org_error_mapper,
        )
}

fn register_member_commands(
    builder: CommandRegistryBuilder,
    member_usecase: Arc<dyn MemberUseCase>,
) -> CommandRegistryBuilder {
    let add_member_handler = Arc::new(AddMemberCommandHandler::new(member_usecase.clone()));
    let remove_member_handler = Arc::new(RemoveMemberCommandHandler::new(member_usecase.clone()));
    let list_members_handler = Arc::new(ListMembersCommandHandler::new(member_usecase.clone()));
    let get_member_handler = Arc::new(GetMemberCommandHandler::new(member_usecase.clone()));
    let update_member_handler = Arc::new(UpdateMemberCommandHandler::new(member_usecase));
    let member_error_mapper = Arc::new(MemberErrorMapper);

    builder
        .register::<AddMemberCommand, _>(
            "add_member".to_string(),
            add_member_handler,
            member_error_mapper.clone(),
        )
        .register::<RemoveMemberCommand, _>(
            "remove_member".to_string(),
            remove_member_handler,
            member_error_mapper.clone(),
        )
        .register::<ListMembersCommand, _>(
            "list_members".to_string(),
            list_members_handler,
            member_error_mapper.clone(),
        )
        .register::<GetMemberCommand, _>(
            "get_member".to_string(),
            get_member_handler,
            member_error_mapper.clone(),
        )
        .register::<UpdateMemberCommand, _>(
            "update_member".to_string(),
            update_member_handler,
            member_error_mapper,
        )
}

fn register_role_commands(
    builder: CommandRegistryBuilder,
    role_usecase: Arc<dyn RoleUseCase>,
) -> CommandRegistryBuilder {
    let list_roles_handler = Arc::new(ListRolesCommandHandler::new(role_usecase.clone()));
    let get_role_handler = Arc::new(GetRoleCommandHandler::new(role_usecase));
    let role_error_mapper = Arc::new(RoleErrorMapper);

    builder
        .register::<ListRolesCommand, _>(
            "list_roles".to_string(),
            list_roles_handler,
            role_error_mapper.clone(),
        )
        .register::<GetRoleCommand, _>("get_role".to_string(), get_role_handler, role_error_mapper)
}

fn register_invitation_commands(
    builder: CommandRegistryBuilder,
    invitation_usecase: Arc<dyn InvitationUseCase>,
) -> CommandRegistryBuilder {
    let create_invitation_handler = Arc::new(CreateInvitationCommandHandler::new(
        invitation_usecase.clone(),
    ));
    let cancel_invitation_handler = Arc::new(CancelInvitationCommandHandler::new(
        invitation_usecase.clone(),
    ));
    let accept_invitation_handler =
        Arc::new(AcceptInvitationCommandHandler::new(invitation_usecase));
    let invitation_error_mapper = Arc::new(InvitationErrorMapper);

    builder
        .register::<CreateInvitationCommand, _>(
            "create_invitation".to_string(),
            create_invitation_handler,
            invitation_error_mapper.clone(),
        )
        .register::<CancelInvitationCommand, _>(
            "cancel_invitation".to_string(),
            cancel_invitation_handler,
            invitation_error_mapper.clone(),
        )
        .register::<AcceptInvitationCommand, _>(
            "accept_invitation".to_string(),
            accept_invitation_handler,
            invitation_error_mapper,
        )
}

fn register_external_link_commands(
    builder: CommandRegistryBuilder,
    external_link_usecase: Arc<dyn ExternalLinkUseCase>,
) -> CommandRegistryBuilder {
    let create_external_link_handler =
        Arc::new(CreateExternalLinkCommandHandler::new(external_link_usecase));
    let external_link_error_mapper = Arc::new(ExternalLinkErrorMapper);

    builder.register::<CreateExternalLinkCommand, _>(
        "create_external_link".to_string(),
        create_external_link_handler,
        external_link_error_mapper,
    )
}

fn register_sync_job_commands(
    builder: CommandRegistryBuilder,
    sync_job_usecase: Arc<dyn SyncJobUseCase>,
) -> CommandRegistryBuilder {
    let start_sync_job_handler = Arc::new(StartSyncJobCommandHandler::new(sync_job_usecase));
    let sync_job_error_mapper = Arc::new(SyncJobErrorMapper);

    builder.register::<StartSyncJobCommand, _>(
        "start_sync_job".to_string(),
        start_sync_job_handler,
        sync_job_error_mapper,
    )
}

impl HiveCommandRegistryFactory {
    /// Create an empty registry builder for custom command registration
    #[must_use]
    pub fn create_empty_builder() -> CommandRegistryBuilder {
        CommandRegistryBuilder::new()
    }

    /// Create a registry builder with only organization commands
    pub fn create_builder_with_organizations(
        organization_usecase: Arc<dyn OrganizationUseCase>,
    ) -> CommandRegistryBuilder {
        register_organization_commands(CommandRegistryBuilder::new(), organization_usecase)
    }

    /// Create a registry builder with only member commands
    pub fn create_builder_with_members(
        member_usecase: Arc<dyn MemberUseCase>,
    ) -> CommandRegistryBuilder {
        let add_member_handler = Arc::new(AddMemberCommandHandler::new(member_usecase.clone()));
        let remove_member_handler = Arc::new(RemoveMemberCommandHandler::new(member_usecase));
        let member_error_mapper = Arc::new(MemberErrorMapper);

        CommandRegistryBuilder::new()
            .register::<AddMemberCommand, _>(
                "add_member".to_string(),
                add_member_handler,
                member_error_mapper.clone(),
            )
            .register::<RemoveMemberCommand, _>(
                "remove_member".to_string(),
                remove_member_handler,
                member_error_mapper,
            )
    }

    /// Create a registry builder with only invitation commands
    pub fn create_builder_with_invitations(
        invitation_usecase: Arc<dyn InvitationUseCase>,
    ) -> CommandRegistryBuilder {
        let create_invitation_handler =
            Arc::new(CreateInvitationCommandHandler::new(invitation_usecase));
        let invitation_error_mapper = Arc::new(InvitationErrorMapper);

        CommandRegistryBuilder::new().register::<CreateInvitationCommand, _>(
            "create_invitation".to_string(),
            create_invitation_handler,
            invitation_error_mapper,
        )
    }

    /// Create a registry builder with only external link commands
    pub fn create_builder_with_external_links(
        external_link_usecase: Arc<dyn ExternalLinkUseCase>,
    ) -> CommandRegistryBuilder {
        register_external_link_commands(CommandRegistryBuilder::new(), external_link_usecase)
    }

    /// Create a registry builder with only sync job commands
    pub fn create_builder_with_sync_jobs(
        sync_job_usecase: Arc<dyn SyncJobUseCase>,
    ) -> CommandRegistryBuilder {
        register_sync_job_commands(CommandRegistryBuilder::new(), sync_job_usecase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_builder() {
        let builder = HiveCommandRegistryFactory::create_empty_builder();
        let registry = builder.build();
        let command_types = registry.list_command_types();

        assert!(command_types.is_empty());
    }
}
