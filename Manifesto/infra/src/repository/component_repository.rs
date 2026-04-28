use async_trait::async_trait;
use manifesto_domain::entity::ProjectComponent;
use manifesto_domain::port::{
    ComponentReadRepository, ComponentRepository, ComponentWriteRepository,
};
use manifesto_domain::value_objects::ComponentStatus;
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter,
};
use std::sync::Arc;
use uuid::Uuid;

use super::entity::{prelude::ProjectComponents, project_components};

pub struct ComponentMapper;

impl ComponentMapper {
    pub fn to_domain(model: project_components::Model) -> Result<ProjectComponent, DomainError> {
        Ok(ProjectComponent {
            id: model.id,
            project_id: model.project_id,
            component_type: model.component_type,
            status: ComponentStatus::from_str(&model.status)?,
            added_at: model.added_at.naive_utc().and_utc(),
            configured_at: model.configured_at.map(|dt| dt.naive_utc().and_utc()),
            activated_at: model.activated_at.map(|dt| dt.naive_utc().and_utc()),
            disabled_at: model.disabled_at.map(|dt| dt.naive_utc().and_utc()),
        })
    }

    pub fn to_active_model(component: &ProjectComponent) -> project_components::ActiveModel {
        project_components::ActiveModel {
            id: ActiveValue::Set(component.id),
            project_id: ActiveValue::Set(component.project_id),
            component_type: ActiveValue::Set(component.component_type.clone()),
            status: ActiveValue::Set(component.status.as_str().to_string()),
            added_at: ActiveValue::Set(component.added_at.into()),
            configured_at: ActiveValue::Set(component.configured_at.map(|dt| dt.into())),
            activated_at: ActiveValue::Set(component.activated_at.map(|dt| dt.into())),
            disabled_at: ActiveValue::Set(component.disabled_at.map(|dt| dt.into())),
        }
    }
}

#[derive(Clone)]
pub struct ComponentReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ComponentReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ComponentReadRepository for ComponentReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ProjectComponent>, DomainError> {
        let component = ProjectComponents::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match component {
            Some(model) => Ok(Some(ComponentMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_project(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<ProjectComponent>, DomainError> {
        let components = ProjectComponents::find()
            .filter(project_components::Column::ProjectId.eq(*project_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        components
            .into_iter()
            .map(ComponentMapper::to_domain)
            .collect()
    }

    async fn find_by_project_and_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<Option<ProjectComponent>, DomainError> {
        let component = ProjectComponents::find()
            .filter(project_components::Column::ProjectId.eq(*project_id))
            .filter(project_components::Column::ComponentType.eq(component_type))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match component {
            Some(model) => Ok(Some(ComponentMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn count_active_by_project(&self, project_id: &Uuid) -> Result<i64, DomainError> {
        let count = ProjectComponents::find()
            .filter(project_components::Column::ProjectId.eq(*project_id))
            .filter(project_components::Column::Status.eq("active"))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
}

#[derive(Clone)]
pub struct ComponentWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ComponentWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ComponentWriteRepository for ComponentWriteRepositoryImpl {
    async fn save(&self, component: &ProjectComponent) -> Result<ProjectComponent, DomainError> {
        let exists = ProjectComponents::find_by_id(component.id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();

        if exists {
            let active_model = ComponentMapper::to_active_model(component);
            let result = active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;
            ComponentMapper::to_domain(result)
        } else {
            let active_model = ComponentMapper::to_active_model(component);
            let inserted = active_model
                .insert(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;
            ComponentMapper::to_domain(inserted)
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        let result = ProjectComponents::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found(
                "ProjectComponent",
                &id.to_string(),
            ));
        }

        Ok(())
    }

    async fn exists_by_project_and_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<bool, DomainError> {
        let count = ProjectComponents::find()
            .filter(project_components::Column::ProjectId.eq(*project_id))
            .filter(project_components::Column::ComponentType.eq(component_type))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count > 0)
    }
}

#[derive(Clone)]
pub struct ComponentRepositoryImpl {
    read_repo: Arc<dyn ComponentReadRepository>,
    write_repo: Arc<dyn ComponentWriteRepository>,
}

impl ComponentRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn ComponentReadRepository>,
        write_repo: Arc<dyn ComponentWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl ComponentReadRepository for ComponentRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ProjectComponent>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_project(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<ProjectComponent>, DomainError> {
        self.read_repo.find_by_project(project_id).await
    }

    async fn find_by_project_and_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<Option<ProjectComponent>, DomainError> {
        self.read_repo
            .find_by_project_and_type(project_id, component_type)
            .await
    }

    async fn count_active_by_project(&self, project_id: &Uuid) -> Result<i64, DomainError> {
        self.read_repo.count_active_by_project(project_id).await
    }
}

#[async_trait]
impl ComponentWriteRepository for ComponentRepositoryImpl {
    async fn save(&self, component: &ProjectComponent) -> Result<ProjectComponent, DomainError> {
        self.write_repo.save(component).await
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete(id).await
    }

    async fn exists_by_project_and_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<bool, DomainError> {
        self.write_repo
            .exists_by_project_and_type(project_id, component_type)
            .await
    }
}

impl ComponentRepository for ComponentRepositoryImpl {}
