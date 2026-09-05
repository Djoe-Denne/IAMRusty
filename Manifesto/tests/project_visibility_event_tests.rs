use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use manifesto_application::{
    ApplicationError, ProjectUseCase, ProjectUseCaseImpl, UpdateProjectRequest,
};
use manifesto_configuration::BusinessConfig;
use manifesto_domain::{
    entity::{
        Permission, Project, ProjectComponent, ProjectMember, ProjectMemberRolePermission,
        Resource, RolePermission,
    },
    port::ProjectListFilters,
    service::{ComponentService, MemberService, PermissionService, ProjectService},
    value_objects::{MemberSource, OwnerType, Visibility},
};
use rustycog::core::error::DomainError;
use rustycog::events::{DomainEvent, EventPublisher};
use rustycog::permission::{
    InMemoryPermissionChecker, Permission as FgaPermission, ResourceRef, Subject,
};
use uuid::Uuid;

fn unused<T>(name: &'static str) -> Result<T, DomainError> {
    Err(DomainError::internal_error(name))
}

fn build_project(visibility: Visibility) -> Project {
    let owner_id = Uuid::new_v4();
    Project::builder()
        .name("Visibility Event Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(visibility)
        .build()
        .expect("test project should be valid")
}

#[derive(Clone)]
struct MemoryProjectService {
    project: Arc<Mutex<Project>>,
}

impl MemoryProjectService {
    fn new(project: Project) -> Self {
        Self {
            project: Arc::new(Mutex::new(project)),
        }
    }
}

#[async_trait]
impl ProjectService for MemoryProjectService {
    async fn get_project(&self, id: &Uuid) -> Result<Project, DomainError> {
        let project = self
            .project
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if &project.id == id {
            Ok(project)
        } else {
            Err(DomainError::entity_not_found("Project", &id.to_string()))
        }
    }

    async fn create_project(&self, _project: Project) -> Result<Project, DomainError> {
        unused("create_project")
    }

    async fn update_project(&self, project: Project) -> Result<Project, DomainError> {
        *self.project.lock().unwrap_or_else(PoisonError::into_inner) = project.clone();
        Ok(project)
    }

    async fn delete_project(&self, _id: &Uuid) -> Result<(), DomainError> {
        unused("delete_project")
    }

    async fn list_projects(
        &self,
        _filters: ProjectListFilters,
    ) -> Result<Vec<Project>, DomainError> {
        unused("list_projects")
    }

    async fn count_projects(&self, _filters: ProjectListFilters) -> Result<i64, DomainError> {
        unused("count_projects")
    }

    async fn count_projects_by_owner(
        &self,
        _owner_type: OwnerType,
        _owner_id: Uuid,
    ) -> Result<i64, DomainError> {
        unused("count_projects_by_owner")
    }

    async fn validate_publish(&self, _project_id: &Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

struct UnusedComponentService;

#[async_trait]
impl ComponentService for UnusedComponentService {
    async fn get_component(&self, _id: &Uuid) -> Result<ProjectComponent, DomainError> {
        unused("get_component")
    }

    async fn get_component_by_type(
        &self,
        _project_id: &Uuid,
        _component_type: &str,
    ) -> Result<ProjectComponent, DomainError> {
        unused("get_component_by_type")
    }

    async fn add_component(
        &self,
        _component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        unused("add_component")
    }

    async fn update_component(
        &self,
        _component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        unused("update_component")
    }

    async fn remove_component(&self, _id: &Uuid) -> Result<(), DomainError> {
        unused("remove_component")
    }

    async fn list_components(
        &self,
        _project_id: &Uuid,
    ) -> Result<Vec<ProjectComponent>, DomainError> {
        unused("list_components")
    }

    async fn validate_component_type(&self, _component_type: &str) -> Result<(), DomainError> {
        unused("validate_component_type")
    }

    async fn validate_unique_component(
        &self,
        _project_id: &Uuid,
        _component_type: &str,
    ) -> Result<(), DomainError> {
        unused("validate_unique_component")
    }
}

struct UnusedMemberService;

#[async_trait]
impl MemberService for UnusedMemberService {
    async fn get_member(
        &self,
        _project_id: Uuid,
        _user_id: Uuid,
    ) -> Result<ProjectMember, DomainError> {
        unused("get_member")
    }

    async fn add_member(&self, _member: ProjectMember) -> Result<ProjectMember, DomainError> {
        unused("add_member")
    }

    async fn update_member(&self, _member: ProjectMember) -> Result<ProjectMember, DomainError> {
        unused("update_member")
    }

    async fn remove_member(
        &self,
        _project_id: &Uuid,
        _user_id: &Uuid,
        _grace_period_days: Option<i64>,
    ) -> Result<(), DomainError> {
        unused("remove_member")
    }

    async fn list_members(
        &self,
        _project_id: &Uuid,
        _source: Option<MemberSource>,
        _active_only: bool,
        _page: u32,
        _page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError> {
        unused("list_members")
    }

    async fn count_active_members(&self, _project_id: &Uuid) -> Result<i64, DomainError> {
        unused("count_active_members")
    }

    async fn check_member_exists(
        &self,
        _project_id: &Uuid,
        _user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        unused("check_member_exists")
    }
}

struct UnusedPermissionService;

#[async_trait]
impl PermissionService for UnusedPermissionService {
    async fn get_permission_by_level(&self, _level: &str) -> Result<Permission, DomainError> {
        unused("get_permission_by_level")
    }

    async fn get_all_permissions(&self) -> Result<Vec<Permission>, DomainError> {
        unused("get_all_permissions")
    }

    async fn get_resource(&self, _resource_id: &str) -> Result<Resource, DomainError> {
        unused("get_resource")
    }

    async fn get_all_resources(&self) -> Result<Vec<Resource>, DomainError> {
        unused("get_all_resources")
    }

    async fn create_component_type_resource(
        &self,
        _component_type: &str,
    ) -> Result<Resource, DomainError> {
        unused("create_component_type_resource")
    }

    async fn create_component_instance_resource(
        &self,
        _component_id: &Uuid,
    ) -> Result<Resource, DomainError> {
        unused("create_component_instance_resource")
    }

    async fn delete_component_instance_resource(
        &self,
        _component_id: &Uuid,
    ) -> Result<(), DomainError> {
        unused("delete_component_instance_resource")
    }

    async fn delete_resource(&self, _resource_id: &str) -> Result<(), DomainError> {
        unused("delete_resource")
    }

    async fn get_or_create_role_permission(
        &self,
        _project_id: Uuid,
        _resource_name: &str,
        _permission_level: &str,
    ) -> Result<RolePermission, DomainError> {
        unused("get_or_create_role_permission")
    }

    async fn get_role_permissions_for_project(
        &self,
        _project_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError> {
        unused("get_role_permissions_for_project")
    }

    async fn grant_permission_to_member(
        &self,
        _member_id: &Uuid,
        _role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        unused("grant_permission_to_member")
    }

    async fn revoke_permission_from_member(
        &self,
        _member_id: &Uuid,
        _role_permission_id: &Uuid,
    ) -> Result<(), DomainError> {
        unused("revoke_permission_from_member")
    }

    async fn revoke_all_permissions_from_member(
        &self,
        _member_id: &Uuid,
    ) -> Result<(), DomainError> {
        unused("revoke_all_permissions_from_member")
    }
}

#[derive(Default)]
struct PublisherState {
    event_types: Vec<String>,
}

#[derive(Clone, Default)]
struct RecordingEventPublisher {
    state: Arc<Mutex<PublisherState>>,
}

impl RecordingEventPublisher {
    fn event_types(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .event_types
            .clone()
    }
}

#[async_trait]
impl EventPublisher<DomainError> for RecordingEventPublisher {
    async fn publish(&self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .event_types
            .push(event.event_type().to_string());
        Ok(())
    }

    async fn publish_batch(&self, events: &[Box<dyn DomainEvent>]) -> Result<(), DomainError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        for event in events {
            state.event_types.push(event.event_type().to_string());
        }
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

fn usecase(project: Project, publisher: Arc<RecordingEventPublisher>) -> ProjectUseCaseImpl {
    usecase_with_checker(
        project,
        publisher,
        Arc::new(InMemoryPermissionChecker::default()),
    )
}

fn usecase_with_checker(
    project: Project,
    publisher: Arc<RecordingEventPublisher>,
    checker: Arc<InMemoryPermissionChecker>,
) -> ProjectUseCaseImpl {
    ProjectUseCaseImpl::new(
        Arc::new(MemoryProjectService::new(project)),
        Arc::new(UnusedComponentService),
        Arc::new(UnusedMemberService),
        Arc::new(UnusedPermissionService),
        publisher,
        BusinessConfig::default(),
        checker,
    )
}

fn admin_checker(user_id: Uuid, project_id: Uuid) -> Arc<InMemoryPermissionChecker> {
    let checker = Arc::new(InMemoryPermissionChecker::default());
    checker.allow(
        Subject::new(user_id),
        FgaPermission::Admin,
        ResourceRef::new("project", project_id),
    );
    checker
}

#[tokio::test]
async fn update_visibility_flip_emits_visibility_changed_then_updated() {
    let project = build_project(Visibility::Private);
    let publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = usecase_with_checker(
        project.clone(),
        publisher.clone(),
        admin_checker(project.created_by, project.id),
    );

    usecase
        .update_project(
            project.id,
            &UpdateProjectRequest {
                name: None,
                description: None,
                visibility: Some("public".to_string()),
                external_collaboration_enabled: None,
                data_classification: None,
            },
            project.created_by,
        )
        .await
        .expect("visibility flip should succeed");

    assert_eq!(
        publisher.event_types(),
        ["project_visibility_changed", "project_updated"]
    );
}

#[tokio::test]
async fn update_name_only_does_not_emit_visibility_changed() {
    let project = build_project(Visibility::Private);
    let publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = usecase(project.clone(), publisher.clone());

    usecase
        .update_project(
            project.id,
            &UpdateProjectRequest {
                name: Some("Renamed".to_string()),
                description: None,
                visibility: None,
                external_collaboration_enabled: None,
                data_classification: None,
            },
            project.created_by,
        )
        .await
        .expect("name update should succeed");

    assert_eq!(publisher.event_types(), ["project_updated"]);
}

#[tokio::test]
async fn publish_project_does_not_emit_visibility_changed() {
    let project = build_project(Visibility::Private);
    let publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = usecase(project.clone(), publisher.clone());

    usecase
        .publish_project(project.id, project.created_by)
        .await
        .expect("publish should succeed");

    assert_eq!(publisher.event_types(), ["project_published"]);
}

#[tokio::test]
async fn update_to_public_without_admin_is_denied() {
    let project = build_project(Visibility::Private);
    let publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = usecase(project.clone(), publisher.clone());

    let result = usecase
        .update_project(
            project.id,
            &UpdateProjectRequest {
                name: None,
                description: None,
                visibility: Some("public".to_string()),
                external_collaboration_enabled: None,
                data_classification: None,
            },
            project.created_by,
        )
        .await;

    match result {
        Err(ApplicationError::Domain(DomainError::PermissionDenied { .. })) => {}
        other => panic!("expected permission denied, got {other:?}"),
    }
    assert!(publisher.event_types().is_empty());
}

#[tokio::test]
async fn get_private_is_denied_for_anonymous_and_allowed_for_owner() {
    let project = build_project(Visibility::Private);
    let publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = usecase(project.clone(), publisher);

    match usecase.get_project(project.id, None).await {
        Err(ApplicationError::Domain(DomainError::PermissionDenied { .. })) => {}
        other => panic!("anonymous private GET should be denied, got {other:?}"),
    }

    let visible = usecase
        .get_project(project.id, Some(project.created_by))
        .await
        .expect("owner can read their private project");
    assert_eq!(visible.id, project.id);
}

#[tokio::test]
async fn get_public_is_allowed_for_anonymous() {
    let project = build_project(Visibility::Public);
    let publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = usecase(project.clone(), publisher);

    let visible = usecase
        .get_project(project.id, None)
        .await
        .expect("use case allows world-read when visibility is public");
    assert_eq!(visible.visibility, "public");
}
