//! Database fixtures for Manifesto tests

pub mod projects;
pub mod components;
pub mod members;

use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

pub use projects::*;
pub use components::*;
pub use members::*;

/// Main entry point for DB fixtures
pub struct DbFixtures;

impl DbFixtures {
    /// Create a new project fixture builder
    pub fn project() -> ProjectFixtureBuilder {
        ProjectFixtureBuilder::new()
    }

    /// Create a new component fixture builder
    pub fn component() -> ComponentFixtureBuilder {
        ComponentFixtureBuilder::new()
    }

    /// Create a new member fixture builder
    pub fn member() -> MemberFixtureBuilder {
        MemberFixtureBuilder::new()
    }

    // Helper methods for common test scenarios

    /// Create a project with owner member
    pub async fn create_project_with_owner(
        db: &DatabaseConnection,
        owner_id: Uuid,
    ) -> Result<(ProjectFixture, MemberFixture), DbErr> {
        let project = Self::project()
            .personal(owner_id)
            .commit(Arc::new(db.clone()))
            .await?;

        let member = Self::member()
            .owner(project.id(), owner_id)
            .commit(Arc::new(db.clone()))
            .await?;

        Ok((project, member))
    }

    /// Create a project with owner and component
    pub async fn create_project_with_component(
        db: &DatabaseConnection,
        owner_id: Uuid,
        component_type: &str,
    ) -> Result<(ProjectFixture, MemberFixture, ComponentFixture), DbErr> {
        let (project, member) = Self::create_project_with_owner(db, owner_id).await?;

        let component = Self::component()
            .for_project(project.id())
            .component_type(component_type)
            .commit(Arc::new(db.clone()))
            .await?;

        Ok((project, member, component))
    }
}


