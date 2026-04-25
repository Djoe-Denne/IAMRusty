//! Project fixtures for testing

use chrono::Utc;
use rustycog_testing::db::{CommittedFixture, DbFixture};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types from infra crate
use manifesto_infra::repository::entity::projects::{
    ActiveModel as ProjectActiveModel, Entity as ProjectsEntity, Model as ProjectModel,
};

/// Project fixture wrapper
pub struct ProjectFixture {
    inner: CommittedFixture<ProjectModel>,
}

impl ProjectFixture {
    /// Get the project ID
    pub fn id(&self) -> Uuid {
        self.inner.model.id
    }

    /// Get the project name
    pub fn name(&self) -> &str {
        &self.inner.model.name
    }

    /// Get the project status
    pub fn status(&self) -> &str {
        &self.inner.model.status
    }

    /// Get the project owner ID
    pub fn owner_id(&self) -> Uuid {
        self.inner.model.owner_id
    }

    /// Get the inner model
    pub fn model(&self) -> &ProjectModel {
        &self.inner.model
    }
}

/// Project fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct ProjectFixtureBuilder {
    id: Option<Uuid>,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    owner_type: Option<String>,
    owner_id: Option<Uuid>,
    created_by: Option<Uuid>,
    visibility: Option<String>,
    external_collaboration_enabled: Option<bool>,
    data_classification: Option<String>,
}

impl ProjectFixtureBuilder {
    /// Create a new project fixture builder
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
            description: None,
            status: None,
            owner_type: None,
            owner_id: None,
            created_by: None,
            visibility: None,
            external_collaboration_enabled: None,
            data_classification: None,
        }
    }

    /// Set the project ID
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the project name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the project description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the project status
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Set as draft status
    pub fn draft(mut self) -> Self {
        self.status = Some("draft".to_string());
        self
    }

    /// Set as active status
    pub fn active(mut self) -> Self {
        self.status = Some("active".to_string());
        self
    }

    /// Set as archived status
    pub fn archived(mut self) -> Self {
        self.status = Some("archived".to_string());
        self
    }

    /// Set the owner type
    pub fn owner_type(mut self, owner_type: impl Into<String>) -> Self {
        self.owner_type = Some(owner_type.into());
        self
    }

    /// Set the owner ID
    pub fn owner_id(mut self, owner_id: Uuid) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    /// Set as personal project for the given user
    pub fn personal(mut self, user_id: Uuid) -> Self {
        self.owner_type = Some("personal".to_string());
        self.owner_id = Some(user_id);
        self.created_by = Some(user_id);
        self
    }

    /// Set as organization project
    pub fn organization(mut self, org_id: Uuid, created_by: Uuid) -> Self {
        self.owner_type = Some("organization".to_string());
        self.owner_id = Some(org_id);
        self.created_by = Some(created_by);
        self
    }

    /// Set the created_by user
    pub fn created_by(mut self, created_by: Uuid) -> Self {
        self.created_by = Some(created_by);
        self
    }

    /// Set the visibility
    pub fn visibility(mut self, visibility: impl Into<String>) -> Self {
        self.visibility = Some(visibility.into());
        self
    }

    /// Set as private visibility
    pub fn private(mut self) -> Self {
        self.visibility = Some("private".to_string());
        self
    }

    /// Set as public visibility
    pub fn public(mut self) -> Self {
        self.visibility = Some("public".to_string());
        self
    }

    /// Set external collaboration enabled
    pub fn external_collaboration_enabled(mut self, enabled: bool) -> Self {
        self.external_collaboration_enabled = Some(enabled);
        self
    }

    /// Set the data classification
    pub fn data_classification(mut self, classification: impl Into<String>) -> Self {
        self.data_classification = Some(classification.into());
        self
    }

    /// Commit the project to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<ProjectFixture, DbErr> {
        let now: DateTimeWithTimeZone = Utc::now().into();
        let id = self.id.unwrap_or_else(Uuid::new_v4);
        let owner_id = self.owner_id.unwrap_or_else(Uuid::new_v4);
        let created_by = self.created_by.unwrap_or(owner_id);

        let active_model = ProjectActiveModel {
            id: ActiveValue::Set(id),
            name: ActiveValue::Set(self.name.unwrap_or_else(|| format!("Test Project {}", id))),
            description: ActiveValue::Set(self.description),
            status: ActiveValue::Set(self.status.unwrap_or_else(|| "draft".to_string())),
            owner_type: ActiveValue::Set(self.owner_type.unwrap_or_else(|| "personal".to_string())),
            owner_id: ActiveValue::Set(owner_id),
            created_by: ActiveValue::Set(created_by),
            visibility: ActiveValue::Set(self.visibility.unwrap_or_else(|| "private".to_string())),
            external_collaboration_enabled: ActiveValue::Set(
                self.external_collaboration_enabled.unwrap_or(false),
            ),
            data_classification: ActiveValue::Set(
                self.data_classification
                    .unwrap_or_else(|| "internal".to_string()),
            ),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            published_at: ActiveValue::NotSet,
        };

        let model = active_model.insert(db.as_ref()).await?;
        Ok(ProjectFixture {
            inner: CommittedFixture::new(model),
        })
    }
}

impl Default for ProjectFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}
