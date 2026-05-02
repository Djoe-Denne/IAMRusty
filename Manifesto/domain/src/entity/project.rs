use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::value_objects::{DataClassification, OwnerType, ProjectStatus, Visibility};
use rustycog_core::error::DomainError;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub owner_type: OwnerType,
    pub owner_id: Uuid,
    pub created_by: Uuid,
    pub visibility: Visibility,
    pub external_collaboration_enabled: bool,
    pub data_classification: DataClassification,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

impl Project {
    /// Create a new project builder
    #[must_use]
    pub fn builder() -> ProjectBuilder {
        ProjectBuilder::default()
    }

    /// Validate the project
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::invalid_input("Project name cannot be empty"));
        }

        if self.name.len() > 255 {
            return Err(DomainError::invalid_input(
                "Project name cannot exceed 255 characters",
            ));
        }

        if let Some(desc) = &self.description {
            if desc.len() > 1000 {
                return Err(DomainError::invalid_input(
                    "Project description cannot exceed 1000 characters",
                ));
            }
        }

        Ok(())
    }

    /// Transition the project to a new status
    pub fn transition_status(&mut self, new_status: ProjectStatus) -> Result<(), DomainError> {
        let transitioned = self.status.transition_to(new_status)?;
        self.status = transitioned;
        self.updated_at = Utc::now();

        // Set published_at when transitioning to Active
        if new_status == ProjectStatus::Active && self.published_at.is_none() {
            self.published_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Check if the project can be published
    #[must_use]
    pub fn can_publish(&self) -> bool {
        self.status == ProjectStatus::Draft
    }

    /// Check if the project is public
    #[must_use]
    pub const fn is_public(&self) -> bool {
        self.visibility.is_public()
    }

    /// Update project metadata
    pub fn update_metadata(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        visibility: Option<Visibility>,
        external_collaboration_enabled: Option<bool>,
        data_classification: Option<DataClassification>,
    ) -> Result<(), DomainError> {
        if let Some(new_name) = name {
            if new_name.trim().is_empty() {
                return Err(DomainError::invalid_input("Project name cannot be empty"));
            }
            if new_name.len() > 255 {
                return Err(DomainError::invalid_input(
                    "Project name cannot exceed 255 characters",
                ));
            }
            self.name = new_name;
        }

        if let Some(new_desc) = description {
            if let Some(desc) = &new_desc {
                if desc.len() > 1000 {
                    return Err(DomainError::invalid_input(
                        "Project description cannot exceed 1000 characters",
                    ));
                }
            }
            self.description = new_desc;
        }

        if let Some(new_visibility) = visibility {
            self.visibility = new_visibility;
        }

        if let Some(enabled) = external_collaboration_enabled {
            self.external_collaboration_enabled = enabled;
        }

        if let Some(classification) = data_classification {
            self.data_classification = classification;
        }

        self.updated_at = Utc::now();

        Ok(())
    }
}

#[derive(Default)]
pub struct ProjectBuilder {
    id: Option<Uuid>,
    name: Option<String>,
    description: Option<String>,
    status: Option<ProjectStatus>,
    owner_type: Option<OwnerType>,
    owner_id: Option<Uuid>,
    created_by: Option<Uuid>,
    visibility: Option<Visibility>,
    external_collaboration_enabled: Option<bool>,
    data_classification: Option<DataClassification>,
}

impl ProjectBuilder {
    #[must_use]
    pub const fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    #[must_use]
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    #[must_use]
    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    #[must_use]
    pub const fn status(mut self, status: ProjectStatus) -> Self {
        self.status = Some(status);
        self
    }

    #[must_use]
    pub const fn owner_type(mut self, owner_type: OwnerType) -> Self {
        self.owner_type = Some(owner_type);
        self
    }

    #[must_use]
    pub const fn owner_id(mut self, owner_id: Uuid) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    #[must_use]
    pub const fn created_by(mut self, created_by: Uuid) -> Self {
        self.created_by = Some(created_by);
        self
    }

    #[must_use]
    pub const fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = Some(visibility);
        self
    }

    #[must_use]
    pub const fn external_collaboration_enabled(mut self, enabled: bool) -> Self {
        self.external_collaboration_enabled = Some(enabled);
        self
    }

    #[must_use]
    pub const fn data_classification(mut self, classification: DataClassification) -> Self {
        self.data_classification = Some(classification);
        self
    }

    pub fn build(self) -> Result<Project, DomainError> {
        let now = Utc::now();

        let project = Project {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            name: self
                .name
                .ok_or_else(|| DomainError::invalid_input("Project name is required"))?,
            description: self.description,
            status: self.status.unwrap_or(ProjectStatus::Draft),
            owner_type: self
                .owner_type
                .ok_or_else(|| DomainError::invalid_input("Owner type is required"))?,
            owner_id: self
                .owner_id
                .ok_or_else(|| DomainError::invalid_input("Owner ID is required"))?,
            created_by: self
                .created_by
                .ok_or_else(|| DomainError::invalid_input("Created by is required"))?,
            visibility: self.visibility.unwrap_or(Visibility::Private),
            external_collaboration_enabled: self.external_collaboration_enabled.unwrap_or(false),
            data_classification: self
                .data_classification
                .unwrap_or(DataClassification::Internal),
            created_at: now,
            updated_at: now,
            published_at: None,
        };

        project.validate()?;

        Ok(project)
    }
}
