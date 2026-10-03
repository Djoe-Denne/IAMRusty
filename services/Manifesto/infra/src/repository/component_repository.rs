use async_trait::async_trait;
use manifesto_domain::entity::ProjectComponent;
use manifesto_domain::port::{
    ComponentReadRepository, ComponentRepository, ComponentWriteRepository,
};
use manifesto_domain::value_objects::ComponentStatus;
use rustycog::core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use super::entity::{prelude::ProjectComponents, project_components};

pub struct ComponentMapper;

impl ComponentMapper {
    /// Map a `SeaORM` component row to the domain entity.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `model.status` is not a recognized component status.
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

    #[must_use]
    pub fn to_active_model(component: &ProjectComponent) -> project_components::ActiveModel {
        project_components::ActiveModel {
            id: ActiveValue::Set(component.id),
            project_id: ActiveValue::Set(component.project_id),
            component_type: ActiveValue::Set(component.component_type.clone()),
            status: ActiveValue::Set(component.status.as_str().to_string()),
            added_at: ActiveValue::Set(component.added_at.into()),
            configured_at: ActiveValue::Set(component.configured_at.map(std::convert::Into::into)),
            activated_at: ActiveValue::Set(component.activated_at.map(std::convert::Into::into)),
            disabled_at: ActiveValue::Set(component.disabled_at.map(std::convert::Into::into)),
        }
    }
}

#[derive(Clone)]
pub struct ComponentReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ComponentReadRepositoryImpl {
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
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

        i64::try_from(count)
            .map_err(|_| DomainError::internal_error("active component count does not fit in i64"))
    }
}

#[derive(Clone)]
pub struct ComponentWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ComponentWriteRepositoryImpl {
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Mappe une violation d'unicité DB (T3) vers un conflit 409.
    ///
    /// Sous course réelle, la contrainte `project_components_unique` tranche :
    /// l'écriture fautive devient `ResourceAlreadyExists` (409), pas une
    /// erreur interne (500). Le refus vient de la base, pas d'un `if` applicatif.
    fn map_unique_conflict(err: sea_orm::DbErr, component_type: &str) -> DomainError {
        if crate::apparatus_mapping::is_unique_violation(&err) {
            DomainError::resource_already_exists("Component", component_type)
        } else {
            DomainError::internal_error(&err.to_string())
        }
    }

    /// Persist a component using an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the query, insert, or update fails.
    /// A `UNIQUE` violation on `(project_id, component_type)` maps to
    /// `ResourceAlreadyExists` (HTTP 409).
    pub async fn save_with_connection<C>(
        db: &C,
        component: &ProjectComponent,
    ) -> Result<ProjectComponent, DomainError>
    where
        C: ConnectionTrait,
    {
        let exists = ProjectComponents::find_by_id(component.id)
            .one(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();
        let active_model = ComponentMapper::to_active_model(component);
        let model = if exists {
            active_model
                .update(db)
                .await
                .map_err(|e| Self::map_unique_conflict(e, &component.component_type))?
        } else {
            active_model
                .insert(db)
                .await
                .map_err(|e| Self::map_unique_conflict(e, &component.component_type))?
        };
        ComponentMapper::to_domain(model)
    }

    /// Delete a component using an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the delete fails or the row is missing.
    pub async fn delete_with_connection<C>(db: &C, id: &Uuid) -> Result<(), DomainError>
    where
        C: ConnectionTrait,
    {
        let result = ProjectComponents::delete_by_id(*id)
            .exec(db)
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
}

#[async_trait]
impl ComponentWriteRepository for ComponentWriteRepositoryImpl {
    async fn save(&self, component: &ProjectComponent) -> Result<ProjectComponent, DomainError> {
        Self::save_with_connection(self.db.as_ref(), component).await
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        Self::delete_with_connection(self.db.as_ref(), id).await
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
