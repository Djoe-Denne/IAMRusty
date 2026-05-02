use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use manifesto_application::{
    AddComponentRequest, ApplicationError, ComponentUseCase, ComponentUseCaseImpl,
};
use manifesto_configuration::BusinessConfig;
use manifesto_domain::{
    entity::{
        Permission, Project, ProjectComponent, ProjectMemberRolePermission, Resource,
        RolePermission,
    },
    port::ProjectListFilters,
    service::{ComponentService, PermissionService, ProjectService},
    value_objects::{OwnerType, ProjectStatus, Visibility},
};
use rustycog_core::error::DomainError;
use rustycog_events::{DomainEvent, EventPublisher};
use uuid::Uuid;

fn build_project() -> Project {
    let owner_id = Uuid::new_v4();
    Project::builder()
        .name("Consistency Test Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(Visibility::Private)
        .build()
        .expect("test project should be valid")
}

#[derive(Clone)]
struct StaticProjectService {
    project: Project,
}

impl StaticProjectService {
    const fn new(project: Project) -> Self {
        Self { project }
    }
}

#[async_trait]
impl ProjectService for StaticProjectService {
    async fn get_project(&self, id: &Uuid) -> Result<Project, DomainError> {
        if &self.project.id == id {
            Ok(self.project.clone())
        } else {
            Err(DomainError::entity_not_found("Project", &id.to_string()))
        }
    }

    async fn create_project(&self, _project: Project) -> Result<Project, DomainError> {
        Err(DomainError::internal_error(
            "create_project not used in this test",
        ))
    }

    async fn update_project(&self, _project: Project) -> Result<Project, DomainError> {
        Err(DomainError::internal_error(
            "update_project not used in this test",
        ))
    }

    async fn delete_project(&self, _id: &Uuid) -> Result<(), DomainError> {
        Err(DomainError::internal_error(
            "delete_project not used in this test",
        ))
    }

    async fn list_projects(
        &self,
        _filters: ProjectListFilters,
    ) -> Result<Vec<Project>, DomainError> {
        Ok(vec![self.project.clone()])
    }

    async fn count_projects(
        &self,
        _owner_type: Option<OwnerType>,
        _owner_id: Option<Uuid>,
        _status: Option<ProjectStatus>,
        _search: Option<String>,
        _viewer_user_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        Ok(1)
    }

    async fn count_projects_by_owner(
        &self,
        _owner_type: OwnerType,
        _owner_id: Uuid,
    ) -> Result<i64, DomainError> {
        Ok(1)
    }

    async fn validate_publish(&self, _project_id: &Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct ComponentState {
    current: Option<ProjectComponent>,
    add_calls: usize,
    remove_calls: usize,
}

#[derive(Clone)]
struct MockComponentService {
    state: Arc<Mutex<ComponentState>>,
}

impl MockComponentService {
    fn empty() -> Self {
        Self {
            state: Arc::new(Mutex::new(ComponentState::default())),
        }
    }

    fn with_component(component: ProjectComponent) -> Self {
        Self {
            state: Arc::new(Mutex::new(ComponentState {
                current: Some(component),
                add_calls: 0,
                remove_calls: 0,
            })),
        }
    }

    fn snapshot(&self) -> ComponentState {
        let state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        ComponentState {
            current: state.current.clone(),
            add_calls: state.add_calls,
            remove_calls: state.remove_calls,
        }
    }
}

#[async_trait]
impl ComponentService for MockComponentService {
    async fn get_component(&self, id: &Uuid) -> Result<ProjectComponent, DomainError> {
        let state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        match &state.current {
            Some(component) if &component.id == id => Ok(component.clone()),
            _ => Err(DomainError::entity_not_found(
                "ProjectComponent",
                &id.to_string(),
            )),
        }
    }

    async fn get_component_by_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<ProjectComponent, DomainError> {
        let state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        match &state.current {
            Some(component)
                if &component.project_id == project_id
                    && component.component_type == component_type =>
            {
                Ok(component.clone())
            }
            _ => Err(DomainError::entity_not_found(
                "ProjectComponent",
                &format!("{project_id}/{component_type}"),
            )),
        }
    }

    async fn add_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        state.add_calls += 1;
        state.current = Some(component.clone());
        Ok(component)
    }

    async fn update_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        state.current = Some(component.clone());
        Ok(component)
    }

    async fn remove_component(&self, id: &Uuid) -> Result<(), DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        state.remove_calls += 1;
        match &state.current {
            Some(component) if &component.id == id => {
                state.current = None;
                Ok(())
            }
            _ => Err(DomainError::entity_not_found(
                "ProjectComponent",
                &id.to_string(),
            )),
        }
    }

    async fn list_components(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<ProjectComponent>, DomainError> {
        let state = self
            .state
            .lock()
            .expect("component state mutex should not be poisoned");
        match &state.current {
            Some(component) if &component.project_id == project_id => Ok(vec![component.clone()]),
            _ => Ok(vec![]),
        }
    }

    async fn validate_component_type(&self, _component_type: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn validate_unique_component(
        &self,
        _project_id: &Uuid,
        _component_type: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct PermissionState {
    create_instance_calls: usize,
    delete_instance_calls: usize,
}

#[derive(Clone)]
struct MockPermissionService {
    state: Arc<Mutex<PermissionState>>,
    create_instance_error: Option<String>,
    delete_instance_error: Option<String>,
}

impl MockPermissionService {
    fn with_create_instance_error(error: DomainError) -> Self {
        Self {
            state: Arc::new(Mutex::new(PermissionState::default())),
            create_instance_error: Some(error.to_string()),
            delete_instance_error: None,
        }
    }

    fn with_delete_instance_error(error: DomainError) -> Self {
        Self {
            state: Arc::new(Mutex::new(PermissionState::default())),
            create_instance_error: None,
            delete_instance_error: Some(error.to_string()),
        }
    }

    fn snapshot(&self) -> PermissionState {
        let state = self
            .state
            .lock()
            .expect("permission state mutex should not be poisoned");
        PermissionState {
            create_instance_calls: state.create_instance_calls,
            delete_instance_calls: state.delete_instance_calls,
        }
    }
}

#[async_trait]
impl PermissionService for MockPermissionService {
    async fn get_permission_by_level(&self, _level: &str) -> Result<Permission, DomainError> {
        Err(DomainError::internal_error(
            "get_permission_by_level not used in this test",
        ))
    }

    async fn get_all_permissions(&self) -> Result<Vec<Permission>, DomainError> {
        Err(DomainError::internal_error(
            "get_all_permissions not used in this test",
        ))
    }

    async fn get_resource(&self, _resource_id: &str) -> Result<Resource, DomainError> {
        Err(DomainError::internal_error(
            "get_resource not used in this test",
        ))
    }

    async fn get_all_resources(&self) -> Result<Vec<Resource>, DomainError> {
        Err(DomainError::internal_error(
            "get_all_resources not used in this test",
        ))
    }

    async fn create_component_type_resource(
        &self,
        _component_type: &str,
    ) -> Result<Resource, DomainError> {
        Err(DomainError::internal_error(
            "create_component_type_resource not used in this test",
        ))
    }

    async fn create_component_instance_resource(
        &self,
        component_id: &Uuid,
    ) -> Result<Resource, DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("permission state mutex should not be poisoned");
        state.create_instance_calls += 1;
        match &self.create_instance_error {
            Some(message) => Err(DomainError::internal_error(message)),
            None => Ok(Resource::from(component_id.to_string())),
        }
    }

    async fn delete_component_instance_resource(
        &self,
        _component_id: &Uuid,
    ) -> Result<(), DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("permission state mutex should not be poisoned");
        state.delete_instance_calls += 1;
        match &self.delete_instance_error {
            Some(message) => Err(DomainError::internal_error(message)),
            None => Ok(()),
        }
    }

    async fn delete_resource(&self, _resource_id: &str) -> Result<(), DomainError> {
        Err(DomainError::internal_error(
            "delete_resource not used in this test",
        ))
    }

    async fn get_or_create_role_permission(
        &self,
        _project_id: Uuid,
        _resource_name: &str,
        _permission_level: &str,
    ) -> Result<RolePermission, DomainError> {
        Err(DomainError::internal_error(
            "get_or_create_role_permission not used in this test",
        ))
    }

    async fn get_role_permissions_for_project(
        &self,
        _project_id: &Uuid,
    ) -> Result<Vec<RolePermission>, DomainError> {
        Err(DomainError::internal_error(
            "get_role_permissions_for_project not used in this test",
        ))
    }

    async fn grant_permission_to_member(
        &self,
        _member_id: &Uuid,
        _role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        Err(DomainError::internal_error(
            "grant_permission_to_member not used in this test",
        ))
    }

    async fn revoke_permission_from_member(
        &self,
        _member_id: &Uuid,
        _role_permission_id: &Uuid,
    ) -> Result<(), DomainError> {
        Err(DomainError::internal_error(
            "revoke_permission_from_member not used in this test",
        ))
    }

    async fn revoke_all_permissions_from_member(
        &self,
        _member_id: &Uuid,
    ) -> Result<(), DomainError> {
        Err(DomainError::internal_error(
            "revoke_all_permissions_from_member not used in this test",
        ))
    }
}

#[derive(Default)]
struct PublisherState {
    publish_calls: usize,
}

#[derive(Clone, Default)]
struct RecordingEventPublisher {
    state: Arc<Mutex<PublisherState>>,
}

impl RecordingEventPublisher {
    fn publish_calls(&self) -> usize {
        self.state
            .lock()
            .expect("publisher state mutex should not be poisoned")
            .publish_calls
    }
}

#[async_trait]
impl EventPublisher<DomainError> for RecordingEventPublisher {
    async fn publish(&self, _event: &Box<dyn DomainEvent>) -> Result<(), DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("publisher state mutex should not be poisoned");
        state.publish_calls += 1;
        Ok(())
    }

    async fn publish_batch(&self, events: &Vec<Box<dyn DomainEvent>>) -> Result<(), DomainError> {
        let mut state = self
            .state
            .lock()
            .expect("publisher state mutex should not be poisoned");
        state.publish_calls += events.len();
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

#[tokio::test]
async fn add_component_fails_before_persisting_when_instance_acl_creation_fails() {
    let project = build_project();
    let component_service = Arc::new(MockComponentService::empty());
    let permission_service = Arc::new(MockPermissionService::with_create_instance_error(
        DomainError::internal_error("acl create failed"),
    ));
    let event_publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = ComponentUseCaseImpl::new(
        component_service.clone(),
        Arc::new(StaticProjectService::new(project.clone())),
        permission_service.clone(),
        event_publisher.clone(),
        BusinessConfig::default(),
    );

    let result = usecase
        .add_component(
            project.id,
            &AddComponentRequest {
                component_type: "taskboard".to_string(),
            },
            project.created_by,
        )
        .await;

    match result {
        Err(ApplicationError::Domain(DomainError::Internal { message })) => {
            assert!(message.contains("acl create failed"));
        }
        other => panic!("Expected internal domain error, got {other:?}"),
    }

    let component_snapshot = component_service.snapshot();
    assert!(component_snapshot.current.is_none());
    assert_eq!(component_snapshot.add_calls, 0);

    let permission_snapshot = permission_service.snapshot();
    assert_eq!(permission_snapshot.create_instance_calls, 1);
    assert_eq!(permission_snapshot.delete_instance_calls, 0);

    assert_eq!(event_publisher.publish_calls(), 0);
}

#[tokio::test]
async fn remove_component_restores_component_when_instance_acl_deletion_fails() {
    let project = build_project();
    let existing_component = ProjectComponent::new(project.id, "taskboard".to_string())
        .expect("component should be valid");
    let component_id = existing_component.id;

    let component_service = Arc::new(MockComponentService::with_component(
        existing_component.clone(),
    ));
    let permission_service = Arc::new(MockPermissionService::with_delete_instance_error(
        DomainError::internal_error("acl delete failed"),
    ));
    let event_publisher = Arc::new(RecordingEventPublisher::default());
    let usecase = ComponentUseCaseImpl::new(
        component_service.clone(),
        Arc::new(StaticProjectService::new(project.clone())),
        permission_service.clone(),
        event_publisher.clone(),
        BusinessConfig::default(),
    );

    let result = usecase
        .remove_component(project.id, component_id, project.created_by)
        .await;

    match result {
        Err(ApplicationError::Domain(DomainError::Internal { message })) => {
            assert!(message.contains("acl delete failed"));
        }
        other => panic!("Expected internal domain error, got {other:?}"),
    }

    let component_snapshot = component_service.snapshot();
    assert_eq!(component_snapshot.remove_calls, 1);
    assert_eq!(component_snapshot.add_calls, 1);
    assert_eq!(
        component_snapshot
            .current
            .expect("component should be restored")
            .id,
        component_id
    );

    let permission_snapshot = permission_service.snapshot();
    assert_eq!(permission_snapshot.delete_instance_calls, 1);

    assert_eq!(event_publisher.publish_calls(), 0);
}
