use std::sync::Arc;

use rustycog_command::{CommandRegistry, CommandRegistryBuilder, RegistryConfig};
use rustycog_config::CommandConfig;

use super::{
    // Component commands
    AddComponentCommand,
    AddComponentCommandHandler,
    // Member commands
    AddMemberCommand,
    AddMemberCommandHandler,
    // Project commands
    ArchiveProjectCommand,
    ArchiveProjectCommandHandler,
    ComponentErrorMapper,
    CreateProjectCommand,
    CreateProjectCommandHandler,
    DeleteProjectCommand,
    DeleteProjectCommandHandler,
    GetComponentCommand,
    GetComponentCommandHandler,
    GetMemberCommand,
    GetMemberCommandHandler,
    GetProjectCommand,
    GetProjectCommandHandler,
    GetProjectDetailCommand,
    GetProjectDetailCommandHandler,
    GrantPermissionCommand,
    GrantPermissionCommandHandler,
    ListComponentsCommand,
    ListComponentsCommandHandler,
    ListMembersCommand,
    ListMembersCommandHandler,
    ListProjectsCommand,
    ListProjectsCommandHandler,
    MemberErrorMapper,
    ProjectErrorMapper,
    PublishProjectCommand,
    PublishProjectCommandHandler,
    RemoveComponentCommand,
    RemoveComponentCommandHandler,
    RemoveMemberCommand,
    RemoveMemberCommandHandler,
    RevokePermissionCommand,
    RevokePermissionCommandHandler,
    UpdateComponentStatusCommand,
    UpdateComponentStatusCommandHandler,
    UpdateMemberCommand,
    UpdateMemberCommandHandler,
    UpdateProjectCommand,
    UpdateProjectCommandHandler,
};
use crate::usecase::{ComponentUseCase, MemberUseCase, ProjectUseCase};

pub struct ManifestoCommandRegistryFactory;

impl ManifestoCommandRegistryFactory {
    /// Create a complete command registry with all Manifesto command handlers
    pub fn create_manifesto_registry(
        project_usecase: Arc<dyn ProjectUseCase>,
        component_usecase: Arc<dyn ComponentUseCase>,
        member_usecase: Arc<dyn MemberUseCase>,
        command_config: CommandConfig,
    ) -> CommandRegistry {
        let mut builder = CommandRegistryBuilder::with_config(RegistryConfig::from_retry_config(
            &command_config.retry,
        ));

        // Register project command handlers
        builder = Self::register_project_handlers(builder, project_usecase);

        // Register component command handlers
        builder = Self::register_component_handlers(builder, component_usecase);

        // Register member command handlers
        builder = Self::register_member_handlers(builder, member_usecase);

        builder.build()
    }

    fn register_project_handlers(
        builder: CommandRegistryBuilder,
        project_usecase: Arc<dyn ProjectUseCase>,
    ) -> CommandRegistryBuilder {
        let create_handler = Arc::new(CreateProjectCommandHandler::new(project_usecase.clone()));
        let get_handler = Arc::new(GetProjectCommandHandler::new(project_usecase.clone()));
        let get_detail_handler =
            Arc::new(GetProjectDetailCommandHandler::new(project_usecase.clone()));
        let update_handler = Arc::new(UpdateProjectCommandHandler::new(project_usecase.clone()));
        let delete_handler = Arc::new(DeleteProjectCommandHandler::new(project_usecase.clone()));
        let list_handler = Arc::new(ListProjectsCommandHandler::new(project_usecase.clone()));
        let publish_handler = Arc::new(PublishProjectCommandHandler::new(project_usecase.clone()));
        let archive_handler = Arc::new(ArchiveProjectCommandHandler::new(project_usecase));
        let error_mapper = Arc::new(ProjectErrorMapper);

        builder
            .register::<CreateProjectCommand, _>(
                "create_project".to_string(),
                create_handler,
                error_mapper.clone(),
            )
            .register::<GetProjectCommand, _>(
                "get_project".to_string(),
                get_handler,
                error_mapper.clone(),
            )
            .register::<GetProjectDetailCommand, _>(
                "get_project_detail".to_string(),
                get_detail_handler,
                error_mapper.clone(),
            )
            .register::<UpdateProjectCommand, _>(
                "update_project".to_string(),
                update_handler,
                error_mapper.clone(),
            )
            .register::<DeleteProjectCommand, _>(
                "delete_project".to_string(),
                delete_handler,
                error_mapper.clone(),
            )
            .register::<ListProjectsCommand, _>(
                "list_projects".to_string(),
                list_handler,
                error_mapper.clone(),
            )
            .register::<PublishProjectCommand, _>(
                "publish_project".to_string(),
                publish_handler,
                error_mapper.clone(),
            )
            .register::<ArchiveProjectCommand, _>(
                "archive_project".to_string(),
                archive_handler,
                error_mapper,
            )
    }

    fn register_component_handlers(
        builder: CommandRegistryBuilder,
        component_usecase: Arc<dyn ComponentUseCase>,
    ) -> CommandRegistryBuilder {
        let add_handler = Arc::new(AddComponentCommandHandler::new(component_usecase.clone()));
        let get_handler = Arc::new(GetComponentCommandHandler::new(component_usecase.clone()));
        let list_handler = Arc::new(ListComponentsCommandHandler::new(component_usecase.clone()));
        let update_status_handler = Arc::new(UpdateComponentStatusCommandHandler::new(
            component_usecase.clone(),
        ));
        let remove_handler = Arc::new(RemoveComponentCommandHandler::new(component_usecase));
        let error_mapper = Arc::new(ComponentErrorMapper);

        builder
            .register::<AddComponentCommand, _>(
                "add_component".to_string(),
                add_handler,
                error_mapper.clone(),
            )
            .register::<GetComponentCommand, _>(
                "get_component".to_string(),
                get_handler,
                error_mapper.clone(),
            )
            .register::<ListComponentsCommand, _>(
                "list_components".to_string(),
                list_handler,
                error_mapper.clone(),
            )
            .register::<UpdateComponentStatusCommand, _>(
                "update_component_status".to_string(),
                update_status_handler,
                error_mapper.clone(),
            )
            .register::<RemoveComponentCommand, _>(
                "remove_component".to_string(),
                remove_handler,
                error_mapper,
            )
    }

    fn register_member_handlers(
        builder: CommandRegistryBuilder,
        member_usecase: Arc<dyn MemberUseCase>,
    ) -> CommandRegistryBuilder {
        let add_handler = Arc::new(AddMemberCommandHandler::new(member_usecase.clone()));
        let get_handler = Arc::new(GetMemberCommandHandler::new(member_usecase.clone()));
        let list_handler = Arc::new(ListMembersCommandHandler::new(member_usecase.clone()));
        let update_handler = Arc::new(UpdateMemberCommandHandler::new(member_usecase.clone()));
        let remove_handler = Arc::new(RemoveMemberCommandHandler::new(member_usecase.clone()));
        let grant_permission_handler =
            Arc::new(GrantPermissionCommandHandler::new(member_usecase.clone()));
        let revoke_permission_handler =
            Arc::new(RevokePermissionCommandHandler::new(member_usecase));
        let error_mapper = Arc::new(MemberErrorMapper);

        builder
            .register::<AddMemberCommand, _>(
                "add_member".to_string(),
                add_handler,
                error_mapper.clone(),
            )
            .register::<GetMemberCommand, _>(
                "get_member".to_string(),
                get_handler,
                error_mapper.clone(),
            )
            .register::<ListMembersCommand, _>(
                "list_members".to_string(),
                list_handler,
                error_mapper.clone(),
            )
            .register::<UpdateMemberCommand, _>(
                "update_member".to_string(),
                update_handler,
                error_mapper.clone(),
            )
            .register::<RemoveMemberCommand, _>(
                "remove_member".to_string(),
                remove_handler,
                error_mapper.clone(),
            )
            .register::<GrantPermissionCommand, _>(
                "grant_permission".to_string(),
                grant_permission_handler,
                error_mapper.clone(),
            )
            .register::<RevokePermissionCommand, _>(
                "revoke_permission".to_string(),
                revoke_permission_handler,
                error_mapper,
            )
    }
}
