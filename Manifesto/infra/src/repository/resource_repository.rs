use async_trait::async_trait;
use manifesto_domain::entity::Resource;
use manifesto_domain::port::{ResourceReadRepository, ResourceRepository, ResourceWriteRepository};
use rustycog_core::error::DomainError;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, EntityTrait};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{resources, prelude::Resources};

/// Generate the resource ID for a specific component instance
/// Uses just the UUID since resource_type identifies it as a component_instance
pub fn component_resource_id(component_id: &Uuid) -> String {
    component_id.to_string()
}

pub struct ResourceMapper;

impl ResourceMapper {
    pub fn to_domain(model: resources::Model) -> Result<Resource, DomainError> {
        Ok(Resource {
            name: model.name,
            created_at: Some(model.created_at.naive_utc().and_utc()),
        })
    }
}

#[derive(Clone)]
pub struct ResourceReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ResourceReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_id_with_connection<C>(
        db: &C,
        id: &str,
    ) -> Result<Option<Resource>, DomainError>
    where
        C: ConnectionTrait,
    {
        let resource = Resources::find_by_id(id.to_string())
            .one(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match resource {
            Some(r) => Ok(Some(ResourceMapper::to_domain(r)?)),
            None => Ok(None),
        }
    }
}

#[async_trait]
impl ResourceReadRepository for ResourceReadRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<Option<Resource>, DomainError> {
        debug!("Finding resource by id: {}", id);

        Self::find_by_id_with_connection(self.db.as_ref(), id).await
    }

    async fn find_all(&self) -> Result<Vec<Resource>, DomainError> {
        debug!("Finding all resources");

        let resources = Resources::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        resources
            .into_iter()
            .map(ResourceMapper::to_domain)
            .collect()
    }

    async fn find_by_component_id(&self, component_id: &Uuid) -> Result<Option<Resource>, DomainError> {
        let resource_id = component_resource_id(component_id);
        debug!("Finding resource by component id: {} (resource_id: {})", component_id, resource_id);
        self.find_by_id(&resource_id).await
    }
}

#[derive(Clone)]
pub struct ResourceWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ResourceWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ResourceWriteRepository for ResourceWriteRepositoryImpl {
    async fn create_for_component(&self, component_type: &str) -> Result<Resource, DomainError> {
        debug!("Creating resource for component: {}", component_type);

        let active_model = resources::ActiveModel {
            id: ActiveValue::Set(component_type.to_string()),
            resource_type: ActiveValue::Set("component".to_string()),
            name: ActiveValue::Set(component_type.to_string()),
            created_at: ActiveValue::NotSet,
        };

        let model = active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        ResourceMapper::to_domain(model)
    }

    async fn delete_by_id(&self, id: &str) -> Result<(), DomainError> {
        debug!("Deleting resource by id: {}", id);

        Resources::delete_by_id(id.to_string())
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(())
    }

    async fn create_for_component_instance(&self, component_id: &Uuid) -> Result<Resource, DomainError> {
        let resource_id = component_resource_id(component_id);
        debug!("Creating resource for component instance: {} (resource_id: {})", component_id, resource_id);

        let active_model = resources::ActiveModel {
            id: ActiveValue::Set(resource_id.clone()),
            resource_type: ActiveValue::Set("component_instance".to_string()),
            name: ActiveValue::Set(resource_id),
            created_at: ActiveValue::NotSet,
        };

        let model = active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        ResourceMapper::to_domain(model)
    }
}

#[derive(Clone)]
pub struct ResourceRepositoryImpl {
    read_repo: Arc<dyn ResourceReadRepository>,
    write_repo: Arc<dyn ResourceWriteRepository>,
}

impl ResourceRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn ResourceReadRepository>,
        write_repo: Arc<dyn ResourceWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl ResourceReadRepository for ResourceRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<Option<Resource>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_all(&self) -> Result<Vec<Resource>, DomainError> {
        self.read_repo.find_all().await
    }

    async fn find_by_component_id(&self, component_id: &Uuid) -> Result<Option<Resource>, DomainError> {
        self.read_repo.find_by_component_id(component_id).await
    }
}

#[async_trait]
impl ResourceWriteRepository for ResourceRepositoryImpl {
    async fn create_for_component(&self, component_type: &str) -> Result<Resource, DomainError> {
        self.write_repo.create_for_component(component_type).await
    }

    async fn delete_by_id(&self, id: &str) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }

    async fn create_for_component_instance(&self, component_id: &Uuid) -> Result<Resource, DomainError> {
        self.write_repo.create_for_component_instance(component_id).await
    }
}

impl ResourceRepository for ResourceRepositoryImpl {}

