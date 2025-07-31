//! OrganizationMemberRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{OrganizationMember, MemberStatus};
use hive_domain::error::DomainError;
use hive_domain::port::repository::OrganizationMemberRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, QueryOrder, Set, Order
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::OrganizationMembers,
    organization_members,
};

/// SeaORM implementation of OrganizationMemberRepository
#[derive(Clone)]
pub struct OrganizationMemberRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationMemberRepositoryImpl {
    /// Create a new OrganizationMemberRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain organization member
    fn to_domain(model: organization_members::Model) -> Result<OrganizationMember, DomainError> {
        let status = match model.status.as_str() {
            "Pending" => MemberStatus::Pending,
            "Active" => MemberStatus::Active,
            "Suspended" => MemberStatus::Suspended,
            _ => return Err(DomainError::invalid_input(&format!("Invalid member status: {}", model.status))),
        };

        Ok(OrganizationMember {
            id: model.id,
            organization_id: model.organization_id,
            user_id: model.user_id,
            roles: vec![],
            status,
            invited_by_user_id: model.invited_by_user_id,
            invited_at: model.invited_at,
            joined_at: model.joined_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }

    /// Convert a domain organization member to a database active model
    fn to_active_model(member: &OrganizationMember) -> organization_members::ActiveModel {
        let status_str = match member.status {
            MemberStatus::Pending => "Pending",
            MemberStatus::Active => "Active",
            MemberStatus::Suspended => "Suspended",
        };

        organization_members::ActiveModel {
            id: ActiveValue::Set(member.id),
            organization_id: ActiveValue::Set(member.organization_id),
            user_id: ActiveValue::Set(member.user_id),
            status: ActiveValue::Set(status_str.to_string()),
            invited_by_user_id: ActiveValue::Set(member.invited_by_user_id),
            invited_at: ActiveValue::Set(member.invited_at),
            joined_at: ActiveValue::Set(member.joined_at),
            created_at: ActiveValue::Set(member.created_at),
            updated_at: ActiveValue::Set(member.updated_at),
        }
    }
}

#[async_trait]
impl OrganizationMemberRepository for OrganizationMemberRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationMember>, DomainError> {
        debug!("Finding organization member by ID: {}", id);
        
        let member = OrganizationMembers::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match member {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_organization_and_user(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<OrganizationMember>, DomainError> {
        debug!("Finding organization member by org {} and user {}", organization_id, user_id);
        
        let member = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::UserId.eq(*user_id))
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match member {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_organization(&self, organization_id: &Uuid, page: u32, page_size: u32) -> Result<Vec<OrganizationMember>, DomainError> {
        debug!("Finding organization members by organization: {}", organization_id);
        
        let members = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .order_by(organization_members::Column::CreatedAt, Order::Desc)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in members {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_user(&self, user_id: &Uuid) -> Result<Vec<OrganizationMember>, DomainError> {
        debug!("Finding organization members by user: {}", user_id);
        
        let members = OrganizationMembers::find()
            .filter(organization_members::Column::UserId.eq(*user_id))
            .order_by(organization_members::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in members {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_organization_and_status(
        &self,
        organization_id: &Uuid,
        status: &MemberStatus,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        debug!("Finding organization members by org {} and status {:?}", organization_id, status);
        
        let status_str = match status {
            MemberStatus::Pending => "Pending",
            MemberStatus::Active => "Active",
            MemberStatus::Suspended => "Suspended",
        };

        let members = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::Status.eq(status_str))
            .order_by(organization_members::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in members {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_organization_and_role(
        &self,
        organization_id: &Uuid,
        role_id: &Uuid,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        debug!("Finding organization members by org {} and role {}", organization_id, role_id);
        
        let members = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::RoleId.eq(*role_id))
            .order_by(organization_members::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in members {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn is_member(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        debug!("Checking if user {} is member of org {}", user_id, organization_id);
        
        let count = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::UserId.eq(*user_id))
            .count(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(count > 0)
    }

    async fn save(&self, member: &OrganizationMember) -> Result<OrganizationMember, DomainError> {
        debug!("Saving organization member with ID: {}", member.id);
        
        let active_model = Self::to_active_model(member);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        // Convert the saved active model back to domain model
        let saved_model = organization_members::Model {
            id: result.id.unwrap(),
            organization_id: result.organization_id.unwrap(),
            user_id: result.user_id.unwrap(),
            status: result.status.unwrap(),
            invited_by_user_id: result.invited_by_user_id.unwrap(),
            invited_at: result.invited_at.unwrap(),
            joined_at: result.joined_at.unwrap(),
            created_at: result.created_at.unwrap(),
            updated_at: result.updated_at.unwrap(),
        };

        Self::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization member by ID: {}", id);
        
        let result = OrganizationMembers::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("OrganizationMember", &id.to_string()));
        }

        Ok(())
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization members by organization: {}", organization_id);
        
        let result = OrganizationMembers::delete_many().filter(organization_members::Column::OrganizationId.eq(*organization_id)).exec(self.db.as_ref()).await.map_err(DomainError::from)?;
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting members in organization: {}", organization_id);
        
        let count = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .count(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(count as i64)
    }

    async fn count_active_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting active members in organization: {}", organization_id);
        
        let count = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::Status.eq("Active"))
            .count(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(count as i64)
    }
} 