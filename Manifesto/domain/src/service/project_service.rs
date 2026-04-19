use async_trait::async_trait;
use rustycog_core::error::DomainError;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::Project;
use crate::port::{ComponentReadRepository, ProjectRepository};
use crate::value_objects::{OwnerType, ProjectStatus};

#[async_trait]
pub trait ProjectService: Send + Sync {
    async fn get_project(&self, id: &Uuid) -> Result<Project, DomainError>;
    
    async fn create_project(&self, project: Project) -> Result<Project, DomainError>;
    
    async fn update_project(&self, project: Project) -> Result<Project, DomainError>;
    
    async fn delete_project(&self, id: &Uuid) -> Result<(), DomainError>;
    
    async fn list_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Project>, DomainError>;
    
    async fn count_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
    ) -> Result<i64, DomainError>;

    async fn count_projects_by_owner(
        &self,
        owner_type: OwnerType,
        owner_id: Uuid,
    ) -> Result<i64, DomainError>;
    
    async fn validate_publish(&self, project_id: &Uuid) -> Result<(), DomainError>;
}

pub struct ProjectServiceImpl<PR, CR>
where
    PR: ProjectRepository,
    CR: ComponentReadRepository,
{
    project_repo: Arc<PR>,
    component_repo: Arc<CR>,
}

impl<PR, CR> ProjectServiceImpl<PR, CR>
where
    PR: ProjectRepository,
    CR: ComponentReadRepository,
{
    pub fn new(project_repo: Arc<PR>, component_repo: Arc<CR>) -> Self {
        Self {
            project_repo,
            component_repo,
        }
    }
}

#[async_trait]
impl<PR, CR> ProjectService for ProjectServiceImpl<PR, CR>
where
    PR: ProjectRepository,
    CR: ComponentReadRepository,
{
    async fn get_project(&self, id: &Uuid) -> Result<Project, DomainError> {
        self.project_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Project", &id.to_string()))
    }

    async fn create_project(&self, project: Project) -> Result<Project, DomainError> {
        // Validate project
        project.validate()?;
        
        // Save to repository
        self.project_repo.save(&project).await
    }

    async fn update_project(&self, project: Project) -> Result<Project, DomainError> {
        // Validate project
        project.validate()?;
        
        // Ensure project exists
        if !self.project_repo.exists_by_id(&project.id).await? {
            return Err(DomainError::entity_not_found("Project", &project.id.to_string()));
        }
        
        // Save to repository
        self.project_repo.save(&project).await
    }

    async fn delete_project(&self, id: &Uuid) -> Result<(), DomainError> {
        // Ensure project exists
        if !self.project_repo.exists_by_id(id).await? {
            return Err(DomainError::entity_not_found("Project", &id.to_string()));
        }
        
        self.project_repo.delete_by_id(id).await
    }

    async fn list_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Project>, DomainError> {
        self.project_repo
            .list_with_filters(owner_type, owner_id, status, search, viewer_user_id, page, page_size)
            .await
    }

    async fn count_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        viewer_user_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        self.project_repo
            .count_with_filters(owner_type, owner_id, status, search, viewer_user_id)
            .await
    }

    async fn count_projects_by_owner(
        &self,
        owner_type: OwnerType,
        owner_id: Uuid,
    ) -> Result<i64, DomainError> {
        Ok(self
            .project_repo
            .find_by_owner(owner_type, &owner_id)
            .await?
            .len() as i64)
    }

    async fn validate_publish(&self, project_id: &Uuid) -> Result<(), DomainError> {
        let project = self.get_project(project_id).await?;

        // Check if project is in draft status
        if project.status != ProjectStatus::Draft {
            return Err(DomainError::business_rule_violation(
                "Only draft projects can be published",
            ));
        }

        // Check if project has at least one active component
        let active_count = self.component_repo.count_active_by_project(project_id).await?;
        if active_count == 0 {
            return Err(DomainError::invalid_input(
                "Project must have at least one active component to be published",
            ));
        }

        Ok(())
    }
}

