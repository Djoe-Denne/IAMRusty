//! OrganizationMemberRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{OrganizationMember, MemberStatus};
use rustycog_core::error::DomainError;
use hive_domain::port::repository::{
    OrganizationMemberReadRepository, OrganizationMemberRepository, OrganizationMemberWriteRepository,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::OrganizationMembers,
    organization_members,
};

pub struct OrganizationMemberMapper;

impl OrganizationMemberMapper {
    
    pub fn to_domain(model: organization_members::Model) -> Result<OrganizationMember, DomainError> {
        let status = match model.status.to_lowercase().as_str() {
            "pending" => MemberStatus::Pending,
            "active" => MemberStatus::Active,
            "suspended" => MemberStatus::Suspended,
            _ => return Err(DomainError::invalid_input(&format!("Invalid member status: {}", model.status))),
        };

        Ok(OrganizationMember {
            id: Some(model.id),
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

    pub fn to_active_model(member: &OrganizationMember) -> organization_members::ActiveModel {
        let status_str = match member.status {
            MemberStatus::Pending => "pending",
            MemberStatus::Active => "active",
            MemberStatus::Suspended => "suspended",
        };

        organization_members::ActiveModel {
            id: ActiveValue::Set(member.id.unwrap_or(Uuid::new_v4())),
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

/// Read repository implementation
#[derive(Clone)]
pub struct OrganizationMemberReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationMemberReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }

}

#[async_trait]
impl OrganizationMemberReadRepository for OrganizationMemberReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationMember>, DomainError> {
        debug!("Finding organization member by ID: {}", id);
        
        let member = OrganizationMembers::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match member {
            Some(model) => Ok(Some(OrganizationMemberMapper::to_domain(model)?)),
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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match member {
            Some(model) => Ok(Some(OrganizationMemberMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_organization(&self, organization_id: &Uuid, page: u32, page_size: u32) -> Result<Vec<OrganizationMember>, DomainError> {
        debug!("Finding organization members by organization: {}", organization_id);
        
        let members = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .order_by(organization_members::Column::CreatedAt, Order::Desc)
            .paginate(self.db.as_ref(), page_size as u64)
            .fetch_page(page as u64)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in members {
            result.push(OrganizationMemberMapper::to_domain(model)?);
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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in members {
            result.push(OrganizationMemberMapper::to_domain(model)?);
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
            MemberStatus::Pending => "pending",
            MemberStatus::Active => "active",
            MemberStatus::Suspended => "suspended",
        };

        let members = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::Status.eq(status_str))
            .order_by(organization_members::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in members {
            result.push(OrganizationMemberMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting members in organization: {}", organization_id);
        
        let count = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }

    async fn count_active_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting active members in organization: {}", organization_id);
        
        let count = OrganizationMembers::find()
            .filter(organization_members::Column::OrganizationId.eq(*organization_id))
            .filter(organization_members::Column::Status.eq("active"))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
} 

/// Write repository implementation
#[derive(Clone)]
pub struct OrganizationMemberWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationMemberWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl OrganizationMemberWriteRepository for OrganizationMemberWriteRepositoryImpl {
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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count > 0)
    }

    async fn save(&self, member: &OrganizationMember) -> Result<OrganizationMember, DomainError> {
        debug!("Saving organization member with user id: {:?} for org {}", member.user_id, member.organization_id);
        
        let exists = member.id.is_some() && OrganizationMembers::find_by_id(member.id.unwrap())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();

        if exists {
            // Update
            let active_model = OrganizationMemberMapper::to_active_model(member);
            let result = active_model.save(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

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

            Ok(OrganizationMemberMapper::to_domain(saved_model)?)
        } else {
            // Insert
            let active_model = OrganizationMemberMapper::to_active_model(member);
            let result = active_model.insert(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

            Ok(OrganizationMemberMapper::to_domain(result)?)
        }
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization member by ID: {}", id);
        
        let result = OrganizationMembers::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("OrganizationMember", &id.to_string()));
        }

        Ok(())
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization members by organization: {}", organization_id);
        
        let _result = OrganizationMembers::delete_many().filter(organization_members::Column::OrganizationId.eq(*organization_id)).exec(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;
        Ok(())
    }
}

/// Combined delegating repository
#[derive(Clone)]
pub struct OrganizationMemberRepositoryImpl {
    read_repo: Arc<dyn OrganizationMemberReadRepository>,
    write_repo: Arc<dyn OrganizationMemberWriteRepository>,
}

impl OrganizationMemberRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn OrganizationMemberReadRepository>,
        write_repo: Arc<dyn OrganizationMemberWriteRepository>,
    ) -> Self {
        Self { read_repo, write_repo }
    }
}

#[async_trait]
impl OrganizationMemberReadRepository for OrganizationMemberRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationMember>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_organization_and_user(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<OrganizationMember>, DomainError> {
        self.read_repo
            .find_by_organization_and_user(organization_id, user_id)
            .await
    }

    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        self.read_repo
            .find_by_organization(organization_id, page, page_size)
            .await
    }

    async fn find_by_user(&self, user_id: &Uuid) -> Result<Vec<OrganizationMember>, DomainError> {
        self.read_repo.find_by_user(user_id).await
    }

    async fn find_by_organization_and_status(
        &self,
        organization_id: &Uuid,
        status: &MemberStatus,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        self.read_repo
            .find_by_organization_and_status(organization_id, status)
            .await
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        self.read_repo.count_by_organization(organization_id).await
    }

    async fn count_active_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        self.read_repo
            .count_active_by_organization(organization_id)
            .await
    }
}

#[async_trait]
impl OrganizationMemberWriteRepository for OrganizationMemberRepositoryImpl {
    async fn is_member(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        self.write_repo.is_member(organization_id, user_id).await
    }

    async fn save(&self, member: &OrganizationMember) -> Result<OrganizationMember, DomainError> {
        self.write_repo.save(member).await
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_organization(organization_id).await
    }
}

impl OrganizationMemberRepository for OrganizationMemberRepositoryImpl {}