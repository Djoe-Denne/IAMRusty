//! OrganizationRepository SeaORM implementation with read/write split and combined delegator

use async_trait::async_trait;
use hive_domain::entity::Organization;
use hive_domain::port::repository::{
    OrganizationReadRepository, OrganizationRepository, OrganizationWriteRepository,
};
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Condition, prelude::Expr,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    prelude::{OrganizationMembers, Organizations},
    organization_members, organizations,
};

pub struct OrganizationMapper;

impl OrganizationMapper {
    
    pub fn to_domain(model: organizations::Model) -> Organization {
        Organization {
            id: model.id,
            name: model.name,
            slug: model.slug,
            description: model.description,
            avatar_url: model.avatar_url,
            owner_user_id: model.owner_user_id,
            settings: model.settings,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }

    pub fn to_active_model(organization: &Organization) -> organizations::ActiveModel {
        organizations::ActiveModel {
            id: ActiveValue::Set(organization.id),
            name: ActiveValue::Set(organization.name.clone()),
            slug: ActiveValue::Set(organization.slug.clone()),
            description: ActiveValue::Set(organization.description.clone()),
            avatar_url: ActiveValue::Set(organization.avatar_url.clone()),
            owner_user_id: ActiveValue::Set(organization.owner_user_id),
            settings: ActiveValue::Set(organization.settings.clone()),
            created_at: ActiveValue::Set(organization.created_at),
            updated_at: ActiveValue::Set(organization.updated_at),
        }
    }
}

/// Read-only repository backed by a read connection
#[derive(Clone)]
pub struct OrganizationReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

}

#[async_trait]
impl OrganizationReadRepository for OrganizationReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Organization>, DomainError> {
        debug!("Finding organization by ID: {}", id);
        
        let organization = Organizations::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organization.map(OrganizationMapper::to_domain))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Organization>, DomainError> {
        debug!("Finding organization by slug: {}", slug);
        
        let organization = Organizations::find()
            .filter(organizations::Column::Slug.eq(slug))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organization.map(OrganizationMapper::to_domain))
    }

    async fn find_by_owner(&self, owner_user_id: &Uuid) -> Result<Vec<Organization>, DomainError> {
        debug!("Finding organizations by owner: {}", owner_user_id);
        
        let organizations = Organizations::find()
            .filter(organizations::Column::OwnerUserId.eq(*owner_user_id))
            .order_by(organizations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organizations.into_iter().map(OrganizationMapper::to_domain).collect())
    }

    async fn find_by_user_membership(
        &self,
        user_id: &Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        debug!("Finding organizations by user membership: {} (page: {}, size: {})", 
               user_id, page, page_size);
        
        let organizations = Organizations::find()
            .inner_join(OrganizationMembers)
            .filter(organization_members::Column::UserId.eq(*user_id))
            .filter(organization_members::Column::Status.eq("Active"))
            .order_by(organizations::Column::CreatedAt, Order::Desc)
            .paginate(self.db.as_ref(), page_size as u64)
            .fetch_page(page as u64)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organizations.into_iter().map(OrganizationMapper::to_domain).collect())
    }

    async fn search_by_name(
        &self,
        user_id: Option<Uuid>,
        name_pattern: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        debug!("Searching organizations by name pattern: {} (page: {}, size: {}) for user: {}", 
               name_pattern, page, page_size, user_id.unwrap_or_default());

        let mut query = Organizations::find();
        
        let like_pattern = format!("%{}%", name_pattern);
        
        // Filter public by default using settings JSON column
        let mut access_condition: Condition = Condition::all().add(Expr::cust("settings->>'visibility' = 'Public'"));
        
        if let Some(user_id) = user_id {
            query = query
            .left_join(organization_members::Entity);
            access_condition = Condition::any()
                .add(access_condition)
                .add(organization_members::Column::UserId.eq(user_id));
        } 

        let name_condition = organizations::Column::Name.like(&like_pattern);
        
        let full_condition = Condition::all()
            .add(access_condition)
            .add(name_condition);


        let organizations = query
            .filter(full_condition)
            .order_by(organizations::Column::CreatedAt, Order::Desc)
            .paginate(self.db.as_ref(), page_size as u64)
            .fetch_page(page as u64)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organizations.into_iter().map(OrganizationMapper::to_domain).collect())
    }

    async fn count(&self) -> Result<i64, DomainError> {
        debug!("Counting total organizations");
        
        let count = Organizations::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
}

/// Write repository backed by the write connection
#[derive(Clone)]
pub struct OrganizationWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrganizationWriteRepository for OrganizationWriteRepositoryImpl {
    async fn exists_by_slug(&self, slug: &str) -> Result<bool, DomainError> {
        debug!("Checking if organization exists by slug: {}", slug);
        
        let count = Organizations::find()
            .filter(organizations::Column::Slug.eq(slug))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count > 0)
    }

    async fn save(&self, organization: &Organization) -> Result<Organization, DomainError> {
        debug!("Saving organization with ID: {}", organization.id);
        // Decide whether to insert or update based on existence
        let exists = Organizations::find_by_id(organization.id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();

        if exists {
            // Update
            let active_model = OrganizationMapper::to_active_model(organization);
            let result = active_model
                .save(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            let saved_model = organizations::Model {
                id: result.id.unwrap(),
                name: result.name.unwrap(),
                slug: result.slug.unwrap(),
                description: result.description.unwrap(),
                avatar_url: result.avatar_url.unwrap(),
                owner_user_id: result.owner_user_id.unwrap(),
                settings: result.settings.unwrap(),
                created_at: result.created_at.unwrap(),
                updated_at: result.updated_at.unwrap(),
            };
            Ok(OrganizationMapper::to_domain(saved_model))
        } else {
            // Insert
            let active_model = OrganizationMapper::to_active_model(organization);
            let inserted = organizations::ActiveModel { ..active_model }
                .insert(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;
            Ok(OrganizationMapper::to_domain(inserted))
        }
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization by ID: {}", id);
        
        let result = Organizations::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("Organization", &id.to_string()));
        }

        Ok(())
    }
}

/// Combined repository delegating to read and write repositories
#[derive(Clone)]
pub struct OrganizationRepositoryImpl {
    read_repo: Arc<dyn OrganizationReadRepository>,
    write_repo: Arc<dyn OrganizationWriteRepository>,
}

impl OrganizationRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn OrganizationReadRepository>,
        write_repo: Arc<dyn OrganizationWriteRepository>,
    ) -> Self {
        Self { read_repo, write_repo }
    }
}

#[async_trait]
impl OrganizationReadRepository for OrganizationRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Organization>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Organization>, DomainError> {
        self.read_repo.find_by_slug(slug).await
    }

    async fn find_by_owner(&self, owner_user_id: &Uuid) -> Result<Vec<Organization>, DomainError> {
        self.read_repo.find_by_owner(owner_user_id).await
    }

    async fn find_by_user_membership(
        &self,
        user_id: &Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        self.read_repo
            .find_by_user_membership(user_id, page, page_size)
            .await
    }

    async fn search_by_name(
        &self,
        user_id: Option<Uuid>,
        name_pattern: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        self.read_repo
            .search_by_name(user_id, name_pattern, page, page_size)
            .await
    }

    async fn count(&self) -> Result<i64, DomainError> {
        self.read_repo.count().await
    }
}

#[async_trait]
impl OrganizationWriteRepository for OrganizationRepositoryImpl {
    async fn exists_by_slug(&self, slug: &str) -> Result<bool, DomainError> {
        self.write_repo.exists_by_slug(slug).await
    }

    async fn save(&self, organization: &Organization) -> Result<Organization, DomainError> {
        self.write_repo.save(organization).await
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }
}

impl OrganizationRepository for OrganizationRepositoryImpl {}