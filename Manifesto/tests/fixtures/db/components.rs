//! Component fixtures for testing

use chrono::Utc;
use rustycog_testing::db::{CommittedFixture, DbFixture};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types from infra crate
use manifesto_infra::repository::entity::project_components::{
    ActiveModel as ComponentActiveModel, Entity as ComponentsEntity, Model as ComponentModel,
};
use manifesto_infra::repository::entity::resources::ActiveModel as ResourceActiveModel;

/// Component fixture wrapper
pub struct ComponentFixture {
    inner: CommittedFixture<ComponentModel>,
}

impl ComponentFixture {
    /// Get the component ID
    pub fn id(&self) -> Uuid {
        self.inner.model.id
    }

    /// Get the project ID
    pub fn project_id(&self) -> Uuid {
        self.inner.model.project_id
    }

    /// Get the component type
    pub fn component_type(&self) -> &str {
        &self.inner.model.component_type
    }

    /// Get the component status
    pub fn status(&self) -> &str {
        &self.inner.model.status
    }

    /// Get the inner model
    pub fn model(&self) -> &ComponentModel {
        &self.inner.model
    }
}

/// Component fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct ComponentFixtureBuilder {
    id: Option<Uuid>,
    project_id: Option<Uuid>,
    component_type: Option<String>,
    status: Option<String>,
}

impl ComponentFixtureBuilder {
    /// Create a new component fixture builder
    pub fn new() -> Self {
        Self {
            id: None,
            project_id: None,
            component_type: None,
            status: None,
        }
    }

    /// Set the component ID
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the project ID
    pub fn for_project(mut self, project_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the component type
    pub fn component_type(mut self, component_type: impl Into<String>) -> Self {
        self.component_type = Some(component_type.into());
        self
    }

    /// Set as taskboard component
    pub fn taskboard(mut self) -> Self {
        self.component_type = Some("taskboard".to_string());
        self
    }

    /// Set as wiki component
    pub fn wiki(mut self) -> Self {
        self.component_type = Some("wiki".to_string());
        self
    }

    /// Set as repository component
    pub fn repository(mut self) -> Self {
        self.component_type = Some("repository".to_string());
        self
    }

    /// Set the component status
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Set as pending status
    pub fn pending(mut self) -> Self {
        self.status = Some("pending".to_string());
        self
    }

    /// Set as configured status
    pub fn configured(mut self) -> Self {
        self.status = Some("configured".to_string());
        self
    }

    /// Set as active status
    pub fn active(mut self) -> Self {
        self.status = Some("active".to_string());
        self
    }

    /// Set as disabled status
    pub fn disabled(mut self) -> Self {
        self.status = Some("disabled".to_string());
        self
    }

    /// Commit the component to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<ComponentFixture, DbErr> {
        let now: DateTimeWithTimeZone = Utc::now().into();
        let id = self.id.unwrap_or_else(Uuid::new_v4);

        let project_id = self.project_id.ok_or_else(|| {
            DbErr::Custom("project_id is required for ComponentFixtureBuilder".to_string())
        })?;

        let active_model = ComponentActiveModel {
            id: ActiveValue::Set(id),
            project_id: ActiveValue::Set(project_id),
            component_type: ActiveValue::Set(
                self.component_type
                    .unwrap_or_else(|| "taskboard".to_string()),
            ),
            status: ActiveValue::Set(self.status.unwrap_or_else(|| "pending".to_string())),
            added_at: ActiveValue::Set(now.clone()),
            configured_at: ActiveValue::NotSet,
            activated_at: ActiveValue::NotSet,
            disabled_at: ActiveValue::NotSet,
        };

        let model = active_model.insert(db.as_ref()).await?;

        // Create the specific resource for this component instance
        // Uses just the UUID as the ID (resource_type identifies it as component_instance)
        // This mirrors what the ComponentUseCaseImpl.add_component does in production
        let resource_id = id.to_string();
        let resource_model = ResourceActiveModel {
            id: ActiveValue::Set(resource_id.clone()),
            resource_type: ActiveValue::Set("component_instance".to_string()),
            name: ActiveValue::Set(resource_id),
            created_at: ActiveValue::Set(now),
        };
        resource_model.insert(db.as_ref()).await?;

        Ok(ComponentFixture {
            inner: CommittedFixture::new(model),
        })
    }
}

impl Default for ComponentFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}
