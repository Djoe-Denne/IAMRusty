use async_trait::async_trait;
use chrono::Utc;
use manifesto_configuration::BusinessConfig;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::Project,
    port::ProjectListFilters,
    service::{ComponentService, MemberService, PermissionService, ProjectService},
    value_objects::{
        DataClassification, FieldUpdate, MemberSource, OwnerType, ProjectStatus, Visibility,
    },
    ProjectMember,
};
use manifesto_events::{
    ManifestoDomainEvent, ProjectArchivedEvent, ProjectCreatedEvent, ProjectDeletedEvent,
    ProjectPublishedEvent, ProjectUpdatedEvent, ProjectVisibilityChangedEvent,
};
use rustycog::core::error::DomainError;
use rustycog::events::{DomainEvent, EventPublisher};
use rustycog::permission::{Permission, PermissionChecker, ResourceRef, Subject};

use crate::{
    dto::{
        ComponentResponse, CreateProjectRequest, PaginationRequest, PaginationResponse,
        ProjectDetailResponse, ProjectListResponse, ProjectResponse, UpdateProjectRequest,
    },
    usecase::org_scope::{NoopOrgScopeLookup, OrgScopeLookup},
    usecase::world_read::enforce_world_read_or_principal,
    ApplicationError,
};

/// Persists a project and its owner membership in one unit of work.
#[async_trait]
pub trait ProjectAuthorizationUnitOfWork: Send + Sync {
    /// Create a project together with owner permissions.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if persistence or event recording fails.
    async fn create_project_with_owner_permissions(
        &self,
        project: Project,
        owner_member: ProjectMember,
        owner_resource_names: &[&str],
        event: Box<dyn DomainEvent>,
    ) -> Result<(Project, ProjectMember), ApplicationError>;

    /// Persist a project and record AuthZ-relevant events in the same transaction.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if persistence or event recording fails.
    async fn save_project_with_events(
        &self,
        project: Project,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<Project, ApplicationError>;

    /// Persist a member, grant one permission, and record `MemberAdded` in the same transaction.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if persistence or event recording fails.
    async fn save_member_with_permission_and_event(
        &self,
        member: ProjectMember,
        resource_name: &str,
        permission: &str,
        member_limit: u32,
        event: Box<dyn DomainEvent>,
    ) -> Result<ProjectMember, ApplicationError>;
}

/// Application use cases for projects.
#[async_trait]
pub trait ProjectUseCase: Send + Sync {
    /// Create a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if validation or persistence fails.
    async fn create_project(
        &self,
        request: &CreateProjectRequest,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;

    /// Get a project summary.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the project is missing.
    async fn get_project(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ProjectResponse, ApplicationError>;

    /// Get a project with components.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the project is missing.
    async fn get_project_detail(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ProjectDetailResponse, ApplicationError>;

    /// Update project metadata.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the update is forbidden or persistence fails.
    async fn update_project(
        &self,
        project_id: Uuid,
        request: &UpdateProjectRequest,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;

    /// Delete a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the delete is forbidden or persistence fails.
    async fn delete_project(&self, project_id: Uuid, user_id: Uuid)
        -> Result<(), ApplicationError>;

    /// List projects matching the given filters.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the listing fails.
    async fn list_projects(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        user_id: Option<Uuid>,
        pagination: &PaginationRequest,
    ) -> Result<ProjectListResponse, ApplicationError>;

    /// Publish a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the transition is forbidden or persistence fails.
    async fn publish_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;

    /// Archive a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the transition is forbidden or persistence fails.
    async fn archive_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectResponse, ApplicationError>;
}

/// Default [`ProjectUseCase`] implementation.
pub struct ProjectUseCaseImpl {
    project_service: Arc<dyn ProjectService>,
    component_service: Arc<dyn ComponentService>,
    member_service: Arc<dyn MemberService>,
    permission_service: Arc<dyn PermissionService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    business_config: BusinessConfig,
    project_authorization_uow: Option<Arc<dyn ProjectAuthorizationUnitOfWork>>,
    org_permission_checker: Arc<dyn PermissionChecker>,
    org_scope: Arc<dyn OrgScopeLookup>,
}

impl ProjectUseCaseImpl {
    /// Create a project use case without a dedicated creation unit of work.
    pub fn new(
        project_service: Arc<dyn ProjectService>,
        component_service: Arc<dyn ComponentService>,
        member_service: Arc<dyn MemberService>,
        permission_service: Arc<dyn PermissionService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        business_config: BusinessConfig,
        org_permission_checker: Arc<dyn PermissionChecker>,
    ) -> Self {
        Self {
            project_service,
            component_service,
            member_service,
            permission_service,
            event_publisher,
            business_config,
            project_authorization_uow: None,
            org_permission_checker,
            org_scope: Arc::new(NoopOrgScopeLookup),
        }
    }

    /// Persist project creation through a dedicated unit of work.
    #[must_use]
    pub fn with_project_authorization_uow(
        mut self,
        project_authorization_uow: Arc<dyn ProjectAuthorizationUnitOfWork>,
    ) -> Self {
        self.project_authorization_uow = Some(project_authorization_uow);
        self
    }

    /// Resolve org list scopes from stored OpenFGA tuples.
    #[must_use]
    pub fn with_org_scope(mut self, org_scope: Arc<dyn OrgScopeLookup>) -> Self {
        self.org_scope = org_scope;
        self
    }

    async fn persist_project_with_authz_events(
        &self,
        project: Project,
        authz_events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<Project, ApplicationError> {
        if let Some(uow) = &self.project_authorization_uow {
            if !authz_events.is_empty() {
                return uow.save_project_with_events(project, authz_events).await;
            }
        }
        let updated = self.project_service.update_project(project).await?;
        for event in authz_events {
            self.event_publisher.publish(event.as_ref()).await?;
        }
        Ok(updated)
    }

    fn project_to_response(project: &Project) -> ProjectResponse {
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

        if current_count >= i64::from(limit) {
            return Err(ApplicationError::Validation(format!(
                "Project quota exceeded for {} owner {}",
                owner_type.as_str(),
                owner_id
            )));
        }

        Ok(())
    }

    fn visibility_involves_public(old: Visibility, new: Visibility) -> bool {
        matches!(old, Visibility::Public) || matches!(new, Visibility::Public)
    }

    fn permission_denied(message: &str) -> ApplicationError {
        ApplicationError::from(DomainError::permission_denied(message))
    }

    async fn require_admin_on_project(
        &self,
        user_id: Uuid,
        project_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let allowed = self
            .org_permission_checker
            .check(
                Subject::new(user_id),
                Permission::Admin,
                ResourceRef::new("project", project_id),
            )
            .await
            .map_err(ApplicationError::from)?;
        if !allowed {
            return Err(Self::permission_denied(
                "Admin required to change public visibility",
            ));
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
            OwnerType::Organization => {
                let owner_id = request.owner_id.ok_or_else(|| {
                    ApplicationError::Validation(
                        "owner_id required for organization projects".into(),
                    )
                })?;
                let allowed = self
                    .org_permission_checker
                    .check(
                        Subject::new(user_id),
                        Permission::Write,
                        ResourceRef::new("organization", owner_id),
                    )
                    .await
                    .map_err(ApplicationError::from)?;
                if !allowed {
                    return Err(ApplicationError::from(DomainError::permission_denied(
                        "Not a writer on the target organization",
                    )));
                }
                owner_id
            }
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

        let created_project =
            if let Some(project_authorization_uow) = &self.project_authorization_uow {
                let (created_project, _owner_member) = project_authorization_uow
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
                            "Missing role permission ID for owner resource '{resource}'"
                        ))
                    })?;

                    self.permission_service
                        .grant_permission_to_member(&owner_member.id, &role_permission_id)
                        .await?;
                }

                let domain_ev: Box<dyn DomainEvent> = project_created_event.into();
                self.event_publisher.publish(domain_ev.as_ref()).await?;

                created_project
            };

        Ok(Self::project_to_response(&created_project))
    }

    async fn get_project(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ProjectResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        enforce_world_read_or_principal(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;
        Ok(Self::project_to_response(&project))
    }

    async fn get_project_detail(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ProjectDetailResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        enforce_world_read_or_principal(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;

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
            project: Self::project_to_response(&project),
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
        let old_visibility = project.visibility;

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

        if let Some(new_visibility) = visibility {
            if old_visibility != new_visibility
                && Self::visibility_involves_public(old_visibility, new_visibility)
            {
                self.require_admin_on_project(user_id, project_id).await?;
            }
        }

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
                FieldUpdate::Set(request.description.clone()),
                visibility,
                request.external_collaboration_enabled,
                data_classification,
            )
            .map_err(ApplicationError::from)?;

        let mut authz_events: Vec<Box<dyn DomainEvent>> = Vec::new();
        if let Some(new_visibility) = visibility {
            if old_visibility != new_visibility {
                let visibility_event = ManifestoDomainEvent::ProjectVisibilityChanged(
                    ProjectVisibilityChangedEvent::new(
                        project.id,
                        project.owner_type.as_str().to_string(),
                        project.owner_id,
                        old_visibility.as_str().to_string(),
                        new_visibility.as_str().to_string(),
                        user_id,
                        Utc::now(),
                    )
                    .with_visibility_revision(project.revision),
                );
                authz_events.push(visibility_event.into());
            }
        }

        let updated_project = self
            .persist_project_with_authz_events(project, authz_events)
            .await?;

        let event = ManifestoDomainEvent::ProjectUpdated(ProjectUpdatedEvent::new(
            updated_project.id,
            updated_project.name.clone(),
            updated_fields,
            user_id,
            Utc::now(),
        ));
        let domain_ev: Box<dyn DomainEvent> = event.into();
        self.event_publisher.publish(domain_ev.as_ref()).await?;

        Ok(Self::project_to_response(&updated_project))
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
        let domain_ev: Box<dyn DomainEvent> = event.into();
        self.event_publisher.publish(domain_ev.as_ref()).await?;

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

        let (org_viewer_ids, org_admin_ids) = if let Some(uid) = user_id {
            let (viewer, admin) = tokio::try_join!(
                self.org_scope.org_ids_where_viewer(uid),
                self.org_scope.org_ids_where_admin(uid),
            )?;
            (viewer, admin)
        } else {
            (Vec::new(), Vec::new())
        };

        let filters = ProjectListFilters {
            owner_type,
            owner_id,
            status,
            search: search.clone(),
            viewer_user_id: user_id,
            org_viewer_ids,
            org_admin_ids,
            page,
            page_size,
        };

        let projects = self.project_service.list_projects(filters.clone()).await?;

        let total_count = self.project_service.count_projects(filters).await?;

        let data: Vec<ProjectResponse> = projects.iter().map(Self::project_to_response).collect();

        let consumed = i64::from(page.saturating_add(1).saturating_mul(page_size));
        let has_more = consumed < total_count;
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
        let domain_ev: Box<dyn DomainEvent> = event.into();
        self.event_publisher.publish(domain_ev.as_ref()).await?;

        Ok(Self::project_to_response(&published_project))
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

        let event = ManifestoDomainEvent::ProjectArchived(ProjectArchivedEvent::new(
            project.id,
            project.name.clone(),
            project.owner_type.as_str().to_string(),
            project.owner_id,
            user_id,
            Utc::now(),
        ));
        let archived_project = self
            .persist_project_with_authz_events(project, vec![event.into()])
            .await?;

        Ok(Self::project_to_response(&archived_project))
    }
}
