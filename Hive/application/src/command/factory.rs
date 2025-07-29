use super::{
    organization::{
        CreateOrganizationCommand, CreateOrganizationCommandHandler,
        GetOrganizationCommand, GetOrganizationCommandHandler,
        UpdateOrganizationCommand, UpdateOrganizationCommandHandler,
        DeleteOrganizationCommand, DeleteOrganizationCommandHandler,
        ListOrganizationsCommand, ListOrganizationsCommandHandler,
        SearchOrganizationsCommand, SearchOrganizationsCommandHandler,
        OrganizationErrorMapper,
    },
    member::{
        AddMemberCommand, AddMemberCommandHandler,
        RemoveMemberCommand, RemoveMemberCommandHandler,
        ListMembersCommand, ListMembersCommandHandler,
        GetMemberCommand, GetMemberCommandHandler,
        UpdateMemberCommand, UpdateMemberCommandHandler,
        MemberErrorMapper,
    },
    invitation::{
        CreateInvitationCommand, CreateInvitationCommandHandler,
        ListInvitationsCommand, ListInvitationsCommandHandler,
        CancelInvitationCommand, CancelInvitationCommandHandler,
        AcceptInvitationCommand, AcceptInvitationCommandHandler,
        GetInvitationByTokenCommand, GetInvitationByTokenCommandHandler,
        ResendInvitationCommand, ResendInvitationCommandHandler,
        InvitationErrorMapper,
    },
    external_link::{
        CreateExternalLinkCommand, CreateExternalLinkCommandHandler,
        ExternalLinkErrorMapper,
    },
    sync_job::{
        StartSyncJobCommand, StartSyncJobCommandHandler,
        SyncJobErrorMapper,
    },
};
use crate::usecase::{
    OrganizationUseCase, MemberUseCase, InvitationUseCase, 
    ExternalLinkUseCase, SyncJobUseCase
};
use rustycog_command::{CommandRegistry, CommandRegistryBuilder};
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
    ) -> CommandRegistry {
        let mut builder = CommandRegistryBuilder::new();

        // Register organization commands
        let create_org_handler = Arc::new(CreateOrganizationCommandHandler::new(organization_usecase.clone()));
        let get_org_handler = Arc::new(GetOrganizationCommandHandler::new(organization_usecase.clone()));
        let update_org_handler = Arc::new(UpdateOrganizationCommandHandler::new(organization_usecase.clone()));
        let delete_org_handler = Arc::new(DeleteOrganizationCommandHandler::new(organization_usecase.clone()));
        let list_org_handler = Arc::new(ListOrganizationsCommandHandler::new(organization_usecase.clone()));
        let search_org_handler = Arc::new(SearchOrganizationsCommandHandler::new(organization_usecase));
        let org_error_mapper = Arc::new(OrganizationErrorMapper);

        builder = builder
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
            );

        // Register member commands
        let add_member_handler = Arc::new(AddMemberCommandHandler::new(member_usecase.clone()));
        let remove_member_handler = Arc::new(RemoveMemberCommandHandler::new(member_usecase.clone()));
        let list_members_handler = Arc::new(ListMembersCommandHandler::new(member_usecase.clone()));
        let get_member_handler = Arc::new(GetMemberCommandHandler::new(member_usecase.clone()));
        let update_member_handler = Arc::new(UpdateMemberCommandHandler::new(member_usecase));
        let member_error_mapper = Arc::new(MemberErrorMapper);

        builder = builder
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
            );

        // Register invitation commands
        let create_invitation_handler = Arc::new(CreateInvitationCommandHandler::new(invitation_usecase.clone()));
        let list_invitations_handler = Arc::new(ListInvitationsCommandHandler::new(invitation_usecase.clone()));
        let cancel_invitation_handler = Arc::new(CancelInvitationCommandHandler::new(invitation_usecase.clone()));
        let accept_invitation_handler = Arc::new(AcceptInvitationCommandHandler::new(invitation_usecase.clone()));
        let get_invitation_by_token_handler = Arc::new(GetInvitationByTokenCommandHandler::new(invitation_usecase.clone()));
        let resend_invitation_handler = Arc::new(ResendInvitationCommandHandler::new(invitation_usecase));
        let invitation_error_mapper = Arc::new(InvitationErrorMapper);

        builder = builder
            .register::<CreateInvitationCommand, _>(
                "create_invitation".to_string(),
                create_invitation_handler,
                invitation_error_mapper.clone(),
            )
            .register::<ListInvitationsCommand, _>(
                "list_invitations".to_string(),
                list_invitations_handler,
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
                invitation_error_mapper.clone(),
            )
            .register::<GetInvitationByTokenCommand, _>(
                "get_invitation_by_token".to_string(),
                get_invitation_by_token_handler,
                invitation_error_mapper.clone(),
            )
            .register::<ResendInvitationCommand, _>(
                "resend_invitation".to_string(),
                resend_invitation_handler,
                invitation_error_mapper,
            );

        // Register external link commands
        let create_external_link_handler = Arc::new(CreateExternalLinkCommandHandler::new(external_link_usecase));
        let external_link_error_mapper = Arc::new(ExternalLinkErrorMapper);

        builder = builder
            .register::<CreateExternalLinkCommand, _>(
                "create_external_link".to_string(),
                create_external_link_handler,
                external_link_error_mapper,
            );

        // Register sync job commands
        let start_sync_job_handler = Arc::new(StartSyncJobCommandHandler::new(sync_job_usecase));
        let sync_job_error_mapper = Arc::new(SyncJobErrorMapper);

        builder = builder
            .register::<StartSyncJobCommand, _>(
                "start_sync_job".to_string(),
                start_sync_job_handler,
                sync_job_error_mapper,
            );

        builder.build()
    }

    /// Create an empty registry builder for custom command registration
    pub fn create_empty_builder() -> CommandRegistryBuilder {
        CommandRegistryBuilder::new()
    }

    /// Create a registry builder with only organization commands
    pub fn create_builder_with_organizations(
        organization_usecase: Arc<dyn OrganizationUseCase>,
    ) -> CommandRegistryBuilder {
        let create_org_handler = Arc::new(CreateOrganizationCommandHandler::new(organization_usecase.clone()));
        let get_org_handler = Arc::new(GetOrganizationCommandHandler::new(organization_usecase.clone()));
        let update_org_handler = Arc::new(UpdateOrganizationCommandHandler::new(organization_usecase.clone()));
        let delete_org_handler = Arc::new(DeleteOrganizationCommandHandler::new(organization_usecase.clone()));
        let list_org_handler = Arc::new(ListOrganizationsCommandHandler::new(organization_usecase.clone()));
        let search_org_handler = Arc::new(SearchOrganizationsCommandHandler::new(organization_usecase));
        let org_error_mapper = Arc::new(OrganizationErrorMapper);

        CommandRegistryBuilder::new()
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
        let create_invitation_handler = Arc::new(CreateInvitationCommandHandler::new(invitation_usecase));
        let invitation_error_mapper = Arc::new(InvitationErrorMapper);

        CommandRegistryBuilder::new()
            .register::<CreateInvitationCommand, _>(
                "create_invitation".to_string(),
                create_invitation_handler,
                invitation_error_mapper,
            )
    }

    /// Create a registry builder with only external link commands
    pub fn create_builder_with_external_links(
        external_link_usecase: Arc<dyn ExternalLinkUseCase>,
    ) -> CommandRegistryBuilder {
        let create_external_link_handler = Arc::new(CreateExternalLinkCommandHandler::new(external_link_usecase));
        let external_link_error_mapper = Arc::new(ExternalLinkErrorMapper);

        CommandRegistryBuilder::new()
            .register::<CreateExternalLinkCommand, _>(
                "create_external_link".to_string(),
                create_external_link_handler,
                external_link_error_mapper,
            )
    }

    /// Create a registry builder with only sync job commands
    pub fn create_builder_with_sync_jobs(
        sync_job_usecase: Arc<dyn SyncJobUseCase>,
    ) -> CommandRegistryBuilder {
        let start_sync_job_handler = Arc::new(StartSyncJobCommandHandler::new(sync_job_usecase));
        let sync_job_error_mapper = Arc::new(SyncJobErrorMapper);

        CommandRegistryBuilder::new()
            .register::<StartSyncJobCommand, _>(
                "start_sync_job".to_string(),
                start_sync_job_handler,
                sync_job_error_mapper,
            )
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