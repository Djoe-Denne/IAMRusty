//! Unit tests for public-read ACL behavior and project-list filter wiring

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use manifesto_domain::{
    entity::{Project, ProjectComponent, ProjectMember},
    port::{ComponentReadRepository, ProjectReadRepository, ProjectWriteRepository},
    service::{
        ComponentPermissionFetcher, MemberService, ProjectPermissionFetcher, ProjectService,
        ProjectServiceImpl,
    },
    value_objects::{MemberSource, OwnerType, ProjectStatus, Visibility},
};
use rustycog_core::error::DomainError;
use rustycog_permission::{Permission, PermissionsFetcher, ResourceId};
use uuid::Uuid;

fn build_project(visibility: Visibility) -> Project {
    let owner_id = Uuid::new_v4();
    Project::builder()
        .name("Test Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(visibility)
        .build()
        .expect("test project should be valid")
}

#[derive(Clone)]
struct StaticProjectService {
    project: Project,
}

impl StaticProjectService {
    fn new(project: Project) -> Self {
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
        Err(DomainError::internal_error("create_project not used in this test"))
    }

    async fn update_project(&self, _project: Project) -> Result<Project, DomainError> {
        Err(DomainError::internal_error("update_project not used in this test"))
    }

    async fn delete_project(&self, _id: &Uuid) -> Result<(), DomainError> {
        Err(DomainError::internal_error("delete_project not used in this test"))
    }

    async fn list_projects(
        &self,
        _owner_type: Option<OwnerType>,
        _owner_id: Option<Uuid>,
        _status: Option<ProjectStatus>,
        _search: Option<String>,
        _viewer_user_id: Option<Uuid>,
        _page: u32,
        _page_size: u32,
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

struct DummyMemberService;

#[async_trait]
impl MemberService for DummyMemberService {
    async fn get_member(&self, project_id: Uuid, user_id: Uuid) -> Result<ProjectMember, DomainError> {
        Err(DomainError::entity_not_found(
            "ProjectMember",
            &format!("{project_id}/{user_id}"),
        ))
    }

    async fn add_member(&self, _member: ProjectMember) -> Result<ProjectMember, DomainError> {
        Err(DomainError::internal_error("add_member not used in this test"))
    }

    async fn update_member(&self, _member: ProjectMember) -> Result<ProjectMember, DomainError> {
        Err(DomainError::internal_error("update_member not used in this test"))
    }

    async fn remove_member(
        &self,
        _project_id: &Uuid,
        _user_id: &Uuid,
        _grace_period_days: Option<i64>,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_members(
        &self,
        _project_id: &Uuid,
        _source: Option<MemberSource>,
        _active_only: bool,
        _page: u32,
        _page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError> {
        Ok(vec![])
    }

    async fn count_active_members(&self, _project_id: &Uuid) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn check_member_exists(
        &self,
        _project_id: &Uuid,
        _user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedListArgs {
    search: Option<String>,
    viewer_user_id: Option<Uuid>,
    page: u32,
    page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedCountArgs {
    search: Option<String>,
    viewer_user_id: Option<Uuid>,
}

struct RecordingProjectRepository {
    project: Project,
    last_list_args: Mutex<Option<RecordedListArgs>>,
    last_count_args: Mutex<Option<RecordedCountArgs>>,
}

impl RecordingProjectRepository {
    fn new(project: Project) -> Self {
        Self {
            project,
            last_list_args: Mutex::new(None),
            last_count_args: Mutex::new(None),
        }
    }

    fn last_list_args(&self) -> RecordedListArgs {
        self.last_list_args
            .lock()
            .expect("list args mutex should not be poisoned")
            .clone()
            .expect("list args should be recorded")
    }

    fn last_count_args(&self) -> RecordedCountArgs {
        self.last_count_args
            .lock()
            .expect("count args mutex should not be poisoned")
            .clone()
            .expect("count args should be recorded")
    }
}

#[async_trait]
impl ProjectReadRepository for RecordingProjectRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Project>, DomainError> {
        if &self.project.id == id {
            Ok(Some(self.project.clone()))
        } else {
            Ok(None)
        }
    }

    async fn find_by_owner(
        &self,
        _owner_type: OwnerType,
        _owner_id: &Uuid,
    ) -> Result<Vec<Project>, DomainError> {
        Ok(vec![])
    }

    async fn list_with_filters(
        &self,
        _owner_type: Option<OwnerType>,
        _owner_id: Option<Uuid>,
        _status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Project>, DomainError> {
        *self
            .last_list_args
            .lock()
            .expect("list args mutex should not be poisoned") = Some(RecordedListArgs {
            search,
            viewer_user_id,
            page,
            page_size,
        });

        Ok(vec![self.project.clone()])
    }

    async fn count(&self) -> Result<i64, DomainError> {
        Ok(1)
    }

    async fn count_with_filters(
        &self,
        _owner_type: Option<OwnerType>,
        _owner_id: Option<Uuid>,
        _status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        *self
            .last_count_args
            .lock()
            .expect("count args mutex should not be poisoned") = Some(RecordedCountArgs {
            search,
            viewer_user_id,
        });

        Ok(1)
    }
}

#[async_trait]
impl ProjectWriteRepository for RecordingProjectRepository {
    async fn save(&self, project: &Project) -> Result<Project, DomainError> {
        Ok(project.clone())
    }

    async fn delete_by_id(&self, _id: &Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn exists_by_id(&self, id: &Uuid) -> Result<bool, DomainError> {
        Ok(&self.project.id == id)
    }
}

impl manifesto_domain::port::ProjectRepository for RecordingProjectRepository {}

struct NoopComponentReadRepository;

#[async_trait]
impl ComponentReadRepository for NoopComponentReadRepository {
    async fn find_by_id(&self, _id: &Uuid) -> Result<Option<ProjectComponent>, DomainError> {
        Ok(None)
    }

    async fn find_by_project(&self, _project_id: &Uuid) -> Result<Vec<ProjectComponent>, DomainError> {
        Ok(vec![])
    }

    async fn find_by_project_and_type(
        &self,
        _project_id: &Uuid,
        _component_type: &str,
    ) -> Result<Option<ProjectComponent>, DomainError> {
        Ok(None)
    }

    async fn count_active_by_project(&self, _project_id: &Uuid) -> Result<i64, DomainError> {
        Ok(0)
    }
}

#[tokio::test]
async fn anonymous_users_get_read_permission_for_public_projects() {
    let project = build_project(Visibility::Public);
    let fetcher = ProjectPermissionFetcher::new(
        Arc::new(StaticProjectService::new(project.clone())),
        Arc::new(DummyMemberService),
    );

    let permissions = fetcher
        .fetch_permissions(None, vec![ResourceId::from(project.id)])
        .await
        .expect("permission fetch should succeed");

    assert_eq!(permissions, vec![Permission::Read]);
}

#[tokio::test]
async fn anonymous_users_get_no_project_permission_for_private_projects() {
    let project = build_project(Visibility::Private);
    let fetcher = ProjectPermissionFetcher::new(
        Arc::new(StaticProjectService::new(project.clone())),
        Arc::new(DummyMemberService),
    );

    let permissions = fetcher
        .fetch_permissions(None, vec![ResourceId::from(project.id)])
        .await
        .expect("permission fetch should succeed");

    assert!(permissions.is_empty());
}

#[tokio::test]
async fn component_permissions_respect_public_visibility_for_anonymous_users() {
    let public_project = build_project(Visibility::Public);
    let private_project = build_project(Visibility::Private);

    let public_fetcher = ComponentPermissionFetcher::new(
        Arc::new(StaticProjectService::new(public_project.clone())),
        Arc::new(DummyMemberService),
    );
    let private_fetcher = ComponentPermissionFetcher::new(
        Arc::new(StaticProjectService::new(private_project.clone())),
        Arc::new(DummyMemberService),
    );

    let public_permissions = public_fetcher
        .fetch_permissions(
            None,
            vec![ResourceId::from(public_project.id), ResourceId::new_v4()],
        )
        .await
        .expect("public component fetch should succeed");
    let private_permissions = private_fetcher
        .fetch_permissions(
            None,
            vec![ResourceId::from(private_project.id), ResourceId::new_v4()],
        )
        .await
        .expect("private component fetch should succeed");

    assert_eq!(public_permissions, vec![Permission::Read]);
    assert!(private_permissions.is_empty());
}

#[tokio::test]
async fn project_service_forwards_search_and_viewer_filters_to_the_repository() {
    let project = build_project(Visibility::Public);
    let project_repo = Arc::new(RecordingProjectRepository::new(project));
    let service = ProjectServiceImpl::new(project_repo.clone(), Arc::new(NoopComponentReadRepository));
    let viewer_user_id = Uuid::new_v4();

    let listed_projects = service
        .list_projects(
            None,
            None,
            None,
            Some("roadmap".to_string()),
            Some(viewer_user_id),
            2,
            25,
        )
        .await
        .expect("list_projects should succeed");
    let total = service
        .count_projects(
            None,
            None,
            None,
            Some("roadmap".to_string()),
            Some(viewer_user_id),
        )
        .await
        .expect("count_projects should succeed");

    assert_eq!(listed_projects.len(), 1);
    assert_eq!(total, 1);
    assert_eq!(
        project_repo.last_list_args(),
        RecordedListArgs {
            search: Some("roadmap".to_string()),
            viewer_user_id: Some(viewer_user_id),
            page: 2,
            page_size: 25,
        }
    );
    assert_eq!(
        project_repo.last_count_args(),
        RecordedCountArgs {
            search: Some("roadmap".to_string()),
            viewer_user_id: Some(viewer_user_id),
        }
    );
}
