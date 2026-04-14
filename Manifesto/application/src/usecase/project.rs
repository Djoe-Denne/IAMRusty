use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::Project,
    service::{ComponentService, MemberService, PermissionService, ProjectService},
    value_objects::{DataClassification, MemberSource, OwnerType, ProjectStatus, Visibility},
    ProjectMember,
};
use manifesto_events::{
    ManifestoDomainEvent, ProjectArchivedEvent, ProjectCreatedEvent, ProjectDeletedEvent,
    ProjectPublishedEvent, ProjectUpdatedEvent,
};
use rustycog_core::error::DomainError;
use rustycog_events::EventPublisher;

use crate::{
    dto::{
        CreateProjectRequest, PaginationRequest, ProjectDetailResponse, ProjectListResponse,
        ProjectResponse, UpdateProjectRequest, ComponentResponse, PaginationResponse,
    },
    ApplicationError,
};

#[async_trait]
pub trait ProjectUseCase: Send + Sync {
    async fn create_project(
        &self,
        request: &CreateProjectRequest,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;

    async fn get_project(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ProjectResponse, ApplicationError>;

    async fn get_project_detail(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ProjectDetailResponse, ApplicationError>;

    async fn update_project(
        &self,
        project_id: Uuid,
        request: &UpdateProjectRequest,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;

    async fn delete_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    async fn list_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        pagination: &PaginationRequest,
    ) -> Result<ProjectListResponse, ApplicationError>;

    async fn publish_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;

    async fn archive_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;
}

pub struct ProjectUseCaseImpl {
    project_service: Arc<dyn ProjectService>,
    component_service: Arc<dyn ComponentService>,
    member_service: Arc<dyn MemberService>,
    permission_service: Arc<dyn PermissionService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl ProjectUseCaseImpl {
    pub fn new(
        project_service: Arc<dyn ProjectService>,
        component_service: Arc<dyn ComponentService>,
        member_service: Arc<dyn MemberService>,
        permission_service: Arc<dyn PermissionService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            project_service,
            component_service,
            member_service,
            permission_service,
            event_publisher,
        }
    }

    fn project_to_response(&self, project: &Project) -> ProjectResponse {
        ProjectResponse {
            id: project.id,
            name: project.name.clone(),
            description: project.description.clone(),
            status: project.status.as_str().to_string(),
            owner_type: project.owner_type.as_str().to_string(),
            owner_id: project.owner_id,
            created_by: project.created_by,
            visibility: project.visibility.as_str().to_string(),
            external_collaboration_enabled: project.external_collaboration_enabled,
            data_classification: project.data_classification.as_str().to_string(),
            created_at: project.created_at,
            updated_at: project.updated_at,
            published_at: project.published_at,
        }
    }
}

#[async_trait]
impl ProjectUseCase for ProjectUseCaseImpl {
    async fn create_project(
        &self,
        request: &CreateProjectRequest,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError> {
        // Parse and validate inputs
        let owner_type = OwnerType::from_str(&request.owner_type)
            .map_err(ApplicationError::from)?;

        let owner_id = match owner_type {
            OwnerType::Personal => user_id,
            OwnerType::Organization => request.owner_id.ok_or_else(|| {
                ApplicationError::Validation("owner_id required for organization projects".into())
            })?,
        };

        let visibility = request
            .visibility
            .as_ref()
            .map(|v| Visibility::from_str(v))
            .transpose()
            .map_err(ApplicationError::from)?
            .unwrap_or(Visibility::Private);

        let data_classification = request
            .data_classification
            .as_ref()
            .map(|d| DataClassification::from_str(d))
            .transpose()
            .map_err(ApplicationError::from)?
            .unwrap_or(DataClassification::Internal);

        // Build project using domain entity builder
        let project = Project::builder()
            .name(request.name.clone())
            .description(request.description.clone())
            .owner_type(owner_type)
            .owner_id(owner_id)
            .created_by(user_id)
            .visibility(visibility)
            .external_collaboration_enabled(request.external_collaboration_enabled.unwrap_or(false))
            .data_classification(data_classification)
            .build()
            .map_err(ApplicationError::from)?;

        // Create project through service
        let created_project = self.project_service.create_project(project).await?;

        // Create owner member
        let owner_member = ProjectMember::new(
            created_project.id,
            user_id,
            MemberSource::Direct,
            Some(user_id),
        );

        let owner_member = self.member_service.add_member(owner_member).await?;

        for resource in ["project", "component", "member"] {
            let role_permission = self
                .permission_service
                .get_or_create_role_permission(created_project.id, resource, "owner")
                .await?;
            let role_permission_id = role_permission.id.ok_or_else(|| {
                ApplicationError::Internal(format!(
                    "Missing role permission ID for owner resource '{}'",
                    resource
                ))
            })?;

            self.permission_service
                .grant_permission_to_member(&owner_member.id, &role_permission_id)
                .await?;
        }

        // Publish ProjectCreated event
        let event = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
            created_project.id,
            created_project.name.clone(),
            created_project.owner_type.as_str().to_string(),
            created_project.owner_id,
            user_id,
            created_project.visibility.as_str().to_string(),
            created_project.created_at,
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ProjectCreated event: {:?}", e);
        }

        Ok(self.project_to_response(&created_project))
    }

    async fn get_project(
        &self,
        project_id: Uuid,
        _user_id: Option<Uuid>,
    ) -> Result<ProjectResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        Ok(self.project_to_response(&project))
    }

    async fn get_project_detail(
        &self,
        project_id: Uuid,
        _user_id: Option<Uuid>,
    ) -> Result<ProjectDetailResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        
        // Get components from component service
        let domain_components = self.component_service.list_components(&project_id).await?;
        let components: Vec<ComponentResponse> = domain_components
            .iter()
            .map(|c| ComponentResponse {
                id: c.id,
                component_type: c.component_type.clone(),
                status: c.status.as_str().to_string(),
                endpoint: None,
                access_token: None,
                added_at: c.added_at,
                configured_at: c.configured_at,
                activated_at: c.activated_at,
                disabled_at: c.disabled_at,
            })
            .collect();
        
        // Get member count
        let member_count = self.member_service.count_active_members(&project_id).await?;

        Ok(ProjectDetailResponse {
            project: self.project_to_response(&project),
            components,
            member_count,
        })
    }

    async fn update_project(
        &self,
        project_id: Uuid,
        request: &UpdateProjectRequest,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError> {
        let mut project = self.project_service.get_project(&project_id).await?;

        let visibility = request
            .visibility
            .as_ref()
            .map(|v| Visibility::from_str(v))
            .transpose()
            .map_err(ApplicationError::from)?;

        let data_classification = request
            .data_classification
            .as_ref()
            .map(|d| DataClassification::from_str(d))
            .transpose()
            .map_err(ApplicationError::from)?;

        // Track which fields are being updated
        let mut updated_fields = Vec::new();
        if request.name.is_some() {
            updated_fields.push("name".to_string());
        }
        if request.description.is_some() {
            updated_fields.push("description".to_string());
        }
        if request.visibility.is_some() {
            updated_fields.push("visibility".to_string());
        }
        if request.external_collaboration_enabled.is_some() {
            updated_fields.push("external_collaboration_enabled".to_string());
        }
        if request.data_classification.is_some() {
            updated_fields.push("data_classification".to_string());
        }

        project.update_metadata(
            request.name.clone(),
            Some(request.description.clone()),
            visibility,
            request.external_collaboration_enabled,
            data_classification,
        ).map_err(ApplicationError::from)?;

        let updated_project = self.project_service.update_project(project).await?;

        // Publish ProjectUpdated event
        let event = ManifestoDomainEvent::ProjectUpdated(ProjectUpdatedEvent::new(
            updated_project.id,
            updated_project.name.clone(),
            updated_fields,
            user_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ProjectUpdated event: {:?}", e);
        }

        Ok(self.project_to_response(&updated_project))
    }

    async fn delete_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get project before deletion for event data
        let project = self.project_service.get_project(&project_id).await?;
        let project_name = project.name.clone();

        self.project_service.delete_project(&project_id).await?;
        
        // Publish ProjectDeleted event
        let event = ManifestoDomainEvent::ProjectDeleted(ProjectDeletedEvent::new(
            project_id,
            project_name,
            user_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ProjectDeleted event: {:?}", e);
        }

        Ok(())
    }

    async fn list_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        pagination: &PaginationRequest,
    ) -> Result<ProjectListResponse, ApplicationError> {
        let page = pagination.page();
        let page_size = pagination.page_size();

        let projects = self
            .project_service
            .list_projects(owner_type, owner_id, status, search, page, page_size)
            .await?;

        let total_count = self
            .project_service
            .count_projects(owner_type, owner_id, status)
            .await?;

        let data: Vec<ProjectResponse> = projects
            .iter()
            .map(|p| self.project_to_response(p))
            .collect();

        let has_more = (page + 1) * page_size < total_count as u32;
        let next_cursor = if has_more {
            Some((page + 1).to_string())
        } else {
            None
        };

        let pagination_response = PaginationResponse::new(next_cursor, has_more, Some(total_count));

        Ok(ProjectListResponse {
            data,
            pagination: pagination_response,
        })
    }

    async fn publish_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError> {
        // Validate can publish (checks for active components)
        self.project_service.validate_publish(&project_id).await?;

        let mut project = self.project_service.get_project(&project_id).await?;

        // Transition status
        project.transition_status(ProjectStatus::Active)
            .map_err(ApplicationError::from)?;

        let published_project = self.project_service.update_project(project).await?;

        // Publish ProjectPublished event
        let event = ManifestoDomainEvent::ProjectPublished(ProjectPublishedEvent::new(
            published_project.id,
            published_project.name.clone(),
            user_id,
            published_project.published_at.unwrap_or_else(Utc::now),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ProjectPublished event: {:?}", e);
        }

        Ok(self.project_to_response(&published_project))
    }

    async fn archive_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError> {
        let mut project = self.project_service.get_project(&project_id).await?;

        project.transition_status(ProjectStatus::Archived)
            .map_err(ApplicationError::from)?;

        let archived_project = self.project_service.update_project(project).await?;

        // Publish ProjectArchived event
        let event = ManifestoDomainEvent::ProjectArchived(ProjectArchivedEvent::new(
            archived_project.id,
            archived_project.name.clone(),
            user_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ProjectArchived event: {:?}", e);
        }

        Ok(self.project_to_response(&archived_project))
    }
}
