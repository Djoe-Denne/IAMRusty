//! OrganizationInvitationRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{OrganizationInvitation, InvitationStatus};
use hive_domain::error::DomainError;
use hive_domain::port::repository::OrganizationInvitationRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, QueryOrder, Set, Order
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::OrganizationInvitations,
    organization_invitations,
};

/// SeaORM implementation of OrganizationInvitationRepository
#[derive(Clone)]
pub struct OrganizationInvitationRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationInvitationRepositoryImpl {
    /// Create a new OrganizationInvitationRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain organization invitation
    fn to_domain(model: organization_invitations::Model) -> Result<OrganizationInvitation, DomainError> {
        let status = match model.status.as_str() {
            "Pending" => InvitationStatus::Pending,
            "Accepted" => InvitationStatus::Accepted,
            "Expired" => InvitationStatus::Expired,
            "Cancelled" => InvitationStatus::Cancelled,
            _ => return Err(DomainError::invalid_input(&format!("Invalid invitation status: {}", model.status))),
        };

        Ok(OrganizationInvitation {
            id: model.id,
            organization_id: model.organization_id,
            aggregate_id: model.aggregate_id,
            role_permissions: model.role_permissions.try_into(),
            invited_by_user_id: model.invited_by_user_id,
            token: model.token,
            status,
            expires_at: model.expires_at,
            accepted_at: model.accepted_at,
            message: model.message,
            created_at: model.created_at,
        })
    }

    /// Convert a domain organization invitation to a database active model
    fn to_active_model(invitation: &OrganizationInvitation) -> organization_invitations::ActiveModel {
        let status_str = match invitation.status {
            InvitationStatus::Pending => "Pending",
            InvitationStatus::Accepted => "Accepted",
            InvitationStatus::Expired => "Expired",
            InvitationStatus::Cancelled => "Cancelled",
        };

        organization_invitations::ActiveModel {
            id: ActiveValue::Set(invitation.id),
            organization_id: ActiveValue::Set(invitation.organization_id),
            aggregate_id: ActiveValue::Set(invitation.aggregate_id.clone()),
            role_permissions: ActiveValue::Set(Json(invitation.role_permissions)),
            invited_by_user_id: ActiveValue::Set(invitation.invited_by_user_id),
            token: ActiveValue::Set(invitation.token.clone()),
            status: ActiveValue::Set(status_str.to_string()),
            expires_at: ActiveValue::Set(invitation.expires_at),
            accepted_at: ActiveValue::Set(invitation.accepted_at),
            message: ActiveValue::Set(invitation.message.clone()),
            created_at: ActiveValue::Set(invitation.created_at),
        }
    }
}

#[async_trait]
impl OrganizationInvitationRepository for OrganizationInvitationRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitation by ID: {}", id);
        
        let invitation = OrganizationInvitations::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match invitation {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitation by token");
        
        let invitation = OrganizationInvitations::find()
            .filter(organization_invitations::Column::Token.eq(token))
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match invitation {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_organization(&self, organization_id: &Uuid) -> Result<Vec<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitations by organization: {}", organization_id);
        
        let invitations = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .order_by(organization_invitations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_aggregate_id(&self, aggregate_id: &str) -> Result<Vec<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitations by aggregate id: {}", aggregate_id);
        
        let invitations = OrganizationInvitations::find()
            .filter(organization_invitations::Column::AggregateId.eq(aggregate_id))
            .order_by(organization_invitations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_organization_and_aggregate_id_status(
        &self,
        organization_id: &Uuid,
        aggregate_id: &str,
        status: &InvitationStatus,
    ) -> Result<Option<OrganizationInvitation>, DomainError> {
        debug!("Finding invitation by org {} and aggregate id {} and status {:?}", organization_id, aggregate_id, status);
        
        let invitation = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .filter(organization_invitations::Column::AggregateId.eq(aggregate_id))
            .filter(organization_invitations::Column::Status.eq(status))
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match invitation {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_status(&self, status: &InvitationStatus) -> Result<Vec<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitations by status: {:?}", status);
        
        let status_str = match status {
            InvitationStatus::Pending => "Pending",
            InvitationStatus::Accepted => "Accepted",
            InvitationStatus::Expired => "Expired",
            InvitationStatus::Cancelled => "Cancelled",
        };

        let invitations = OrganizationInvitations::find()
            .filter(organization_invitations::Column::Status.eq(status_str))
            .order_by(organization_invitations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_expired(&self) -> Result<Vec<OrganizationInvitation>, DomainError> {
        debug!("Finding expired organization invitations");
        
        let now = Utc::now();
        let invitations = OrganizationInvitations::find()
            .filter(organization_invitations::Column::Status.eq("Pending"))
            .filter(organization_invitations::Column::ExpiresAt.lt(now))
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn save(&self, invitation: &OrganizationInvitation) -> Result<OrganizationInvitation, DomainError> {
        debug!("Saving organization invitation with ID: {}", invitation.id);
        
        let active_model = Self::to_active_model(invitation);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        // Convert the saved active model back to domain model
        let saved_model = organization_invitations::Model {
            id: result.id.unwrap(),
            organization_id: result.organization_id.unwrap(),
            aggregate_id: result.aggregate_id.unwrap(),
            role_permissions: result.role_permissions.unwrap(),
            invited_by_user_id: result.invited_by_user_id.unwrap(),
            token: result.token.unwrap(),
            status: result.status.unwrap(),
            expires_at: result.expires_at.unwrap(),
            accepted_at: result.accepted_at.unwrap(),
            message: result.message.unwrap(),
            created_at: result.created_at.unwrap(),
        };

        Self::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization invitation by ID: {}", id);
        
        let result = OrganizationInvitations::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("OrganizationInvitation", &id.to_string()));
        }

        Ok(())
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting invitations in organization: {}", organization_id);
        
        let count = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .count(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(count as i64)
    }

    async fn count_pending_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting pending invitations in organization: {}", organization_id);
        
        let count = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .filter(organization_invitations::Column::Status.eq("Pending"))
            .count(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(count as i64)
    }
} 