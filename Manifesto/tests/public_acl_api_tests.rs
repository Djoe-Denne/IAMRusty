//! Tests for project-list filter wiring.
//!
//! The old anonymous/public-visibility permission tests that lived here used
//! the removed `ProjectPermissionFetcher` / `ComponentPermissionFetcher`.
//! Public-read semantics are now enforced in the centralized OpenFGA store
//! (public projects get an explicit `project:{id}#viewer@user:*` tuple from
//! the sentinel-sync worker — covered by the worker's translator tests),
//! so this file only keeps the repository-filter-forwarding assertion.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use manifesto_domain::{
    entity::{Project, ProjectComponent},
    port::{ComponentReadRepository, ProjectReadRepository, ProjectWriteRepository},
    service::{ProjectService, ProjectServiceImpl},
    value_objects::{OwnerType, ProjectStatus, Visibility},
};
use rustycog_core::error::DomainError;
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
async fn project_service_forwards_search_and_viewer_filters_to_the_repository() {
    let project = build_project(Visibility::Public);
    let project_repo = Arc::new(RecordingProjectRepository::new(project));
    let service =
        ProjectServiceImpl::new(project_repo.clone(), Arc::new(NoopComponentReadRepository));
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
