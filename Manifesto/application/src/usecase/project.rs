use async_trait::async_trait;
use chrono::Utc;
use manifesto_configuration::BusinessConfig;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    ProjectMember,
    entity::Project,
    service::{ComponentService, MemberService, PermissionService, ProjectService},
    value_objects::{DataClassification, MemberSource, OwnerType, ProjectStatus, Visibility},
};
use manifesto_events::{
    ManifestoDomainEvent, ProjectArchivedEvent, ProjectCreatedEvent, ProjectDeletedEvent,
    ProjectPublishedEvent, ProjectUpdatedEvent,
};
use rustycog_core::error::DomainError;
use rustycog_events::{DomainEvent, EventPublisher};

use crate::{
    ApplicationError,
    dto::{
        ComponentResponse, CreateProjectRequest, PaginationRequest, PaginationResponse,
        ProjectDetailResponse, ProjectListResponse, ProjectResponse, UpdateProjectRequest,
    },
};

#[async_trait]
pub trait ProjectCreationUnitOfWork: Send + Sync {
    async fn create_project_with_owner_permissions(
        &self,
        project: Project,
        owner_member: ProjectMember,
        owner_resource_names: &[&str],
        event: Box<dyn DomainEvent>,
    ) -> Result<(Project, ProjectMember), ApplicationError>;
}

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

    async fn delete_project(&self, project_id: Uuid, user_id: Uuid)
    -> Result<(), ApplicationError>;

    async fn list_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        user_id: Option<Uuid>,
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
    business_config: BusinessConfig,
    project_creation_uow: Option<Arc<dyn ProjectCreationUnitOfWork>>,
}

impl ProjectUseCaseImpl {
    pub fn new(
        project_service: Arc<dyn ProjectService>,
        component_service: Arc<dyn ComponentService>,
        member_service: Arc<dyn MemberService>,
        permission_service: Arc<dyn PermissionService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        business_config: BusinessConfig,
    ) -> Self {
        Self {
            project_service,
            component_service,
            member_service,
            permission_service,
            event_publisher,
            business_config,
            project_creation_uow: None,
        }
    }

    pub fn new_with_project_creation_uow(
        project_service: Arc<dyn ProjectService>,
        component_service: Arc<dyn ComponentService>,
        member_service: Arc<dyn MemberService>,
        permission_service: Arc<dyn PermissionService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        business_config: BusinessConfig,
        project_creation_uow: Arc<dyn ProjectCreationUnitOfWork>,
    ) -> Self {
        Self {
            project_service,
            component_service,
            member_service,
            permission_service,
            event_publisher,
            business_config,
            project_creation_uow: Some(project_creation_uow),
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

    fn configured_page_size(&self, pagination: &PaginationRequest) -> u32 {
        pagination.page_size_with_defaults(
            self.business_config.default_page_size,
            self.business_config.max_page_size,
        )
    }

    fn validate_project_lengths(
        &self,
        name: Option<&str>,
        description: Option<&String>,
    ) -> Result<(), ApplicationError> {
        if let Some(name) = name {
            if name.len() > self.business_config.project_name_max_length {
                return Err(ApplicationError::Validation(format!(
                    "Project name cannot exceed {} characters",
                    self.business_config.project_name_max_length
                )));
            }
        }

        if let Some(description) = description {
            if description.len() > self.business_config.project_description_max_length {
                return Err(ApplicationError::Validation(format!(
                    "Project description cannot exceed {} characters",
                    self.business_config.project_description_max_length
                )));
            }
        }

        Ok(())
    }

    async fn enforce_project_quota(
        &self,
        owner_type: OwnerType,
        owner_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let current_count = self
            .project_service
            .count_projects_by_owner(owner_type, owner_id)
            .await?;

        let limit = match owner_type {
            OwnerType::Personal => self.business_config.max_projects_per_user,
            OwnerType::Organization => self.business_config.max_projects_per_org,
        };

        if current_count >= limit as i64 {
            return Err(ApplicationError::Validation(format!(
                "Project quota exceeded for {} owner {}",
                owner_type.as_str(),
                owner_id
            )));
        }

        Ok(())
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
        let owner_type =
            OwnerType::from_str(&request.owner_type).map_err(ApplicationError::from)?;

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

        self.validate_project_lengths(Some(request.name.as_str()), request.description.as_ref())?;
        self.enforce_project_quota(owner_type, owner_id).await?;

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

        let owner_member =
            ProjectMember::new(project.id, user_id, MemberSource::Direct, Some(user_id));

        let project_created_event = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
            project.id,
            project.name.clone(),
            project.owner_type.as_str().to_string(),
            project.owner_id,
            user_id,
            project.visibility.as_str().to_string(),
            project.created_at,
        ));

        let created_project = if let Some(project_creation_uow) = &self.project_creation_uow {
            let (created_project, _owner_member) = project_creation_uow
                .create_project_with_owner_permissions(
                    project,
                    owner_member,
                    &["project", "component", "member"],
                    project_created_event.into(),
                )
                .await?;
            created_project
        } else {
            // Create project through service
            let created_project = self.project_service.create_project(project).await?;

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

            if let Err(e) = self
                .event_publisher
                .publish(&project_created_event.into())
                .await
            {
                tracing::warn!("Failed to publish ProjectCreated event: {:?}", e);
            }

            created_project
        };

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
        let member_count = self
            .member_service
            .count_active_members(&project_id)
            .await?;

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

        self.validate_project_lengths(request.name.as_deref(), request.description.as_ref())?;

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

        project
            .update_metadata(
                request.name.clone(),
                Some(request.description.clone()),
                visibility,
                request.external_collaboration_enabled,
                data_classification,
            )
            .map_err(ApplicationError::from)?;

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
        user_id: Option<Uuid>,
        pagination: &PaginationRequest,
    ) -> Result<ProjectListResponse, ApplicationError> {
        let page = pagination.page();
        let page_size = self.configured_page_size(pagination);

        let projects = self
            .project_service
            .list_projects(
                owner_type,
                owner_id,
                status,
                search.clone(),
                user_id,
                page,
                page_size,
            )
            .await?;

        let total_count = self
            .project_service
            .count_projects(owner_type, owner_id, status, search, user_id)
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
        project
            .transition_status(ProjectStatus::Active)
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

        project
            .transition_status(ProjectStatus::Archived)
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
