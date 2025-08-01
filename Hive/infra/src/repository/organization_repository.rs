//! OrganizationRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{Organization, MemberStatus};
use hive_domain::error::DomainError;
use hive_domain::port::repository::OrganizationRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    PaginatorTrait, QueryFilter, QueryOrder, Set, Order
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::{Organizations, OrganizationMembers},
    organizations, organization_members,
};

/// SeaORM implementation of OrganizationRepository
#[derive(Clone)]
pub struct OrganizationRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationRepositoryImpl {
    /// Create a new OrganizationRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain organization
    fn to_domain(model: organizations::Model) -> Organization {
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

    /// Convert a domain organization to a database active model
    fn to_active_model(organization: &Organization) -> organizations::ActiveModel {
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

#[async_trait]
impl OrganizationRepository for OrganizationRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Organization>, DomainError> {
        debug!("Finding organization by ID: {}", id);
        
        let organization = Organizations::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organization.map(Self::to_domain))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Organization>, DomainError> {
        debug!("Finding organization by slug: {}", slug);
        
        let organization = Organizations::find()
            .filter(organizations::Column::Slug.eq(slug))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organization.map(Self::to_domain))
    }

    async fn find_by_owner(&self, owner_user_id: &Uuid) -> Result<Vec<Organization>, DomainError> {
        debug!("Finding organizations by owner: {}", owner_user_id);
        
        let organizations = Organizations::find()
            .filter(organizations::Column::OwnerUserId.eq(*owner_user_id))
            .order_by(organizations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organizations.into_iter().map(Self::to_domain).collect())
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

        Ok(organizations.into_iter().map(Self::to_domain).collect())
    }

    async fn search_by_name(
        &self,
        name_pattern: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        debug!("Searching organizations by name pattern: {} (page: {}, size: {})", 
               name_pattern, page, page_size);
        
        let like_pattern = format!("%{}%", name_pattern);
        let organizations = Organizations::find()
            .filter(organizations::Column::Name.like(&like_pattern))
            .order_by(organizations::Column::CreatedAt, Order::Desc)
            .paginate(self.db.as_ref(), page_size as u64)
            .fetch_page(page as u64)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(organizations.into_iter().map(Self::to_domain).collect())
    }

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
        
        let active_model = Self::to_active_model(organization);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        // Convert the saved active model back to domain model
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

        Ok(Self::to_domain(saved_model))
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

    async fn count(&self) -> Result<i64, DomainError> {
        debug!("Counting total organizations");
        
        let count = Organizations::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
} 