use async_trait::async_trait;
use manifesto_domain::entity::Project;
use manifesto_domain::port::{
    ProjectReadRepository, ProjectRepository, ProjectWriteRepository,
};
use manifesto_domain::value_objects::{
    DataClassification, OwnerType, ProjectStatus, Visibility,
};
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Condition,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{prelude::Projects, projects};

pub struct ProjectMapper;

impl ProjectMapper {
    pub fn to_domain(model: projects::Model) -> Result<Project, DomainError> {
        Ok(Project {
            id: model.id,
            name: model.name,
            description: model.description,
            status: ProjectStatus::from_str(&model.status)?,
            owner_type: OwnerType::from_str(&model.owner_type)?,
            owner_id: model.owner_id,
            created_by: model.created_by,
            visibility: Visibility::from_str(&model.visibility)?,
            external_collaboration_enabled: model.external_collaboration_enabled,
            data_classification: DataClassification::from_str(&model.data_classification)?,
            created_at: model.created_at.naive_utc().and_utc(),
            updated_at: model.updated_at.naive_utc().and_utc(),
            published_at: model.published_at.map(|dt| dt.naive_utc().and_utc()),
        })
    }

    pub fn to_active_model(project: &Project) -> projects::ActiveModel {
        projects::ActiveModel {
            id: ActiveValue::Set(project.id),
            name: ActiveValue::Set(project.name.clone()),
            description: ActiveValue::Set(project.description.clone()),
            status: ActiveValue::Set(project.status.as_str().to_string()),
            owner_type: ActiveValue::Set(project.owner_type.as_str().to_string()),
            owner_id: ActiveValue::Set(project.owner_id),
            created_by: ActiveValue::Set(project.created_by),
            visibility: ActiveValue::Set(project.visibility.as_str().to_string()),
            external_collaboration_enabled: ActiveValue::Set(project.external_collaboration_enabled),
            data_classification: ActiveValue::Set(project.data_classification.as_str().to_string()),
            created_at: ActiveValue::Set(project.created_at.into()),
            updated_at: ActiveValue::Set(project.updated_at.into()),
            published_at: ActiveValue::Set(project.published_at.map(|dt| dt.into())),
        }
    }
}

#[derive(Clone)]
pub struct ProjectReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ProjectReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectReadRepository for ProjectReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Project>, DomainError> {
        debug!("Finding project by ID: {}", id);

        let project = Projects::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match project {
            Some(model) => Ok(Some(ProjectMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_owner(
        &self,
        owner_type: OwnerType,
        owner_id: &Uuid,
    ) -> Result<Vec<Project>, DomainError> {
        debug!("Finding projects by owner: {} {}", owner_type, owner_id);

        let projects = Projects::find()
            .filter(projects::Column::OwnerType.eq(owner_type.as_str()))
            .filter(projects::Column::OwnerId.eq(*owner_id))
            .order_by(projects::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        projects
            .into_iter()
            .map(ProjectMapper::to_domain)
            .collect()
    }

    async fn list_with_filters(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Project>, DomainError> {
        debug!("Listing projects with filters");

        let mut query = Projects::find();
        let mut conditions = Condition::all();

        if let Some(ot) = owner_type {
            conditions = conditions.add(projects::Column::OwnerType.eq(ot.as_str()));
        }

        if let Some(oid) = owner_id {
            conditions = conditions.add(projects::Column::OwnerId.eq(oid));
        }

        if let Some(s) = status {
            conditions = conditions.add(projects::Column::Status.eq(s.as_str()));
        }

        if let Some(search_term) = search {
            let like_pattern = format!("%{}%", search_term);
            conditions = conditions.add(projects::Column::Name.like(&like_pattern));
        }

        let projects = query
            .filter(conditions)
            .order_by(projects::Column::CreatedAt, Order::Desc)
            .paginate(self.db.as_ref(), page_size as u64)
            .fetch_page(page as u64)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        projects
            .into_iter()
            .map(ProjectMapper::to_domain)
            .collect()
    }

    async fn count(&self) -> Result<i64, DomainError> {
        debug!("Counting total projects");

        let count = Projects::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }

    async fn count_with_filters(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
    ) -> Result<i64, DomainError> {
        debug!("Counting projects with filters");

        let mut query = Projects::find();
        let mut conditions = Condition::all();

        if let Some(ot) = owner_type {
            conditions = conditions.add(projects::Column::OwnerType.eq(ot.as_str()));
        }

        if let Some(oid) = owner_id {
            conditions = conditions.add(projects::Column::OwnerId.eq(oid));
        }

        if let Some(s) = status {
            conditions = conditions.add(projects::Column::Status.eq(s.as_str()));
        }

        let count = query
            .filter(conditions)
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
}

#[derive(Clone)]
pub struct ProjectWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ProjectWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectWriteRepository for ProjectWriteRepositoryImpl {
    async fn save(&self, project: &Project) -> Result<Project, DomainError> {
        debug!("Saving project with ID: {}", project.id);

        let exists = Projects::find_by_id(project.id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();

        if exists {
            // Update
            let active_model = ProjectMapper::to_active_model(project);
            let result = active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            ProjectMapper::to_domain(result)
        } else {
            // Insert
            let active_model = ProjectMapper::to_active_model(project);
            let inserted = active_model
                .insert(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            ProjectMapper::to_domain(inserted)
        }
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting project by ID: {}", id);

        let result = Projects::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("Project", &id.to_string()));
        }

        Ok(())
    }

    async fn exists_by_id(&self, id: &Uuid) -> Result<bool, DomainError> {
        debug!("Checking if project exists by ID: {}", id);

        let count = Projects::find_by_id(*id)
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count > 0)
    }
}

#[derive(Clone)]
pub struct ProjectRepositoryImpl {
    read_repo: Arc<dyn ProjectReadRepository>,
    write_repo: Arc<dyn ProjectWriteRepository>,
}

impl ProjectRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn ProjectReadRepository>,
        write_repo: Arc<dyn ProjectWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl ProjectReadRepository for ProjectRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Project>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_owner(
        &self,
        owner_type: OwnerType,
        owner_id: &Uuid,
    ) -> Result<Vec<Project>, DomainError> {
        self.read_repo.find_by_owner(owner_type, owner_id).await
    }

    async fn list_with_filters(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
        search: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Project>, DomainError> {
        self.read_repo
            .list_with_filters(owner_type, owner_id, status, search, page, page_size)
            .await
    }

    async fn count(&self) -> Result<i64, DomainError> {
        self.read_repo.count().await
    }

    async fn count_with_filters(
        &self,
        owner_type: Option<OwnerType>,
        owner_id: Option<Uuid>,
        status: Option<ProjectStatus>,
    ) -> Result<i64, DomainError> {
        self.read_repo
            .count_with_filters(owner_type, owner_id, status)
            .await
    }
}

#[async_trait]
impl ProjectWriteRepository for ProjectRepositoryImpl {
    async fn save(&self, project: &Project) -> Result<Project, DomainError> {
        self.write_repo.save(project).await
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }

    async fn exists_by_id(&self, id: &Uuid) -> Result<bool, DomainError> {
        self.write_repo.exists_by_id(id).await
    }
}

impl ProjectRepository for ProjectRepositoryImpl {}

