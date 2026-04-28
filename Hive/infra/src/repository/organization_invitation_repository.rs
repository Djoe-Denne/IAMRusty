//! OrganizationInvitationRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{InvitationStatus, OrganizationInvitation};
use hive_domain::port::repository::{
    OrganizationInvitationReadRepository, OrganizationInvitationRepository,
    OrganizationInvitationWriteRepository,
};
use rustycog_core::error::DomainError;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, JsonValue,
    Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{organization_invitations, prelude::OrganizationInvitations};

pub struct OrganizationInvitationMapper;

impl OrganizationInvitationMapper {
    pub fn to_domain(
        model: organization_invitations::Model,
    ) -> Result<OrganizationInvitation, DomainError> {
        let status = match model.status.as_str() {
            "Pending" => InvitationStatus::Pending,
            "Accepted" => InvitationStatus::Accepted,
            "Expired" => InvitationStatus::Expired,
            "Cancelled" => InvitationStatus::Cancelled,
            _ => {
                return Err(DomainError::invalid_input(&format!(
                    "Invalid invitation status: {}",
                    model.status
                )))
            }
        };

        Ok(OrganizationInvitation {
            id: model.id,
            organization_id: model.organization_id,
            organization_name: None,
            aggregate_id: model.aggregate_id,
            role_permissions: serde_json::from_value(model.role_permissions).map_err(|e| {
                DomainError::invalid_input(&format!("Invalid role permissions: {}", e))
            })?,
            invited_by_user_id: model.invited_by_user_id,
            token: model.token,
            status,
            expires_at: model.expires_at,
            accepted_at: model.accepted_at,
            message: model.message,
            created_at: model.created_at,
        })
    }

    pub fn to_active_model(
        invitation: &OrganizationInvitation,
    ) -> organization_invitations::ActiveModel {
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
            role_permissions: ActiveValue::Set(JsonValue::from(
                serde_json::to_value(&invitation.role_permissions).unwrap(),
            )),
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

/// Read repository
#[derive(Clone)]
pub struct OrganizationInvitationReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationInvitationReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrganizationInvitationReadRepository for OrganizationInvitationReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitation by ID: {}", id);

        let invitation = OrganizationInvitations::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match invitation {
            Some(model) => Ok(Some(OrganizationInvitationMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<OrganizationInvitation>, DomainError> {
        debug!("Finding organization invitation by token");

        let invitation = OrganizationInvitations::find()
            .filter(organization_invitations::Column::Token.eq(token))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match invitation {
            Some(model) => Ok(Some(OrganizationInvitationMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        debug!(
            "Finding organization invitations by organization: {}",
            organization_id
        );

        let invitations = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .order_by(organization_invitations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(OrganizationInvitationMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_aggregate_id(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        debug!(
            "Finding organization invitations by aggregate id: {}",
            aggregate_id
        );

        let invitations = OrganizationInvitations::find()
            .filter(organization_invitations::Column::AggregateId.eq(aggregate_id))
            .order_by(organization_invitations::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(OrganizationInvitationMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_organization_and_aggregate_id_status(
        &self,
        organization_id: &Uuid,
        aggregate_id: &str,
        status: &InvitationStatus,
    ) -> Result<Option<OrganizationInvitation>, DomainError> {
        debug!(
            "Finding invitation by org {} and aggregate id {} and status {:?}",
            organization_id, aggregate_id, status
        );

        let invitation = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .filter(organization_invitations::Column::AggregateId.eq(aggregate_id))
            .filter(organization_invitations::Column::Status.eq(status.as_str()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match invitation {
            Some(model) => Ok(Some(OrganizationInvitationMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_status(
        &self,
        status: &InvitationStatus,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(OrganizationInvitationMapper::to_domain(model)?);
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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in invitations {
            result.push(OrganizationInvitationMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting invitations in organization: {}", organization_id);

        let count = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }

    async fn count_pending_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<i64, DomainError> {
        debug!(
            "Counting pending invitations in organization: {}",
            organization_id
        );

        let count = OrganizationInvitations::find()
            .filter(organization_invitations::Column::OrganizationId.eq(*organization_id))
            .filter(organization_invitations::Column::Status.eq("Pending"))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
}

/// Write repository
#[derive(Clone)]
pub struct OrganizationInvitationWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl OrganizationInvitationWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrganizationInvitationWriteRepository for OrganizationInvitationWriteRepositoryImpl {
    async fn save(
        &self,
        invitation: &OrganizationInvitation,
    ) -> Result<OrganizationInvitation, DomainError> {
        debug!("Saving organization invitation with ID: {}", invitation.id);

        let active_model = OrganizationInvitationMapper::to_active_model(invitation);

        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

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

        OrganizationInvitationMapper::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting organization invitation by ID: {}", id);

        let result = OrganizationInvitations::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found(
                "OrganizationInvitation",
                &id.to_string(),
            ));
        }

        Ok(())
    }
}

/// Combined delegator
#[derive(Clone)]
pub struct OrganizationInvitationRepositoryImpl {
    read_repo: Arc<dyn OrganizationInvitationReadRepository>,
    write_repo: Arc<dyn OrganizationInvitationWriteRepository>,
}

impl OrganizationInvitationRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn OrganizationInvitationReadRepository>,
        write_repo: Arc<dyn OrganizationInvitationWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl OrganizationInvitationReadRepository for OrganizationInvitationRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<OrganizationInvitation>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_token(
        &self,
        token: &str,
    ) -> Result<Option<OrganizationInvitation>, DomainError> {
        self.read_repo.find_by_token(token).await
    }

    async fn find_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        self.read_repo.find_by_organization(organization_id).await
    }

    async fn find_by_aggregate_id(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        self.read_repo.find_by_aggregate_id(aggregate_id).await
    }

    async fn find_by_organization_and_aggregate_id_status(
        &self,
        organization_id: &Uuid,
        aggregate_id: &str,
        status: &InvitationStatus,
    ) -> Result<Option<OrganizationInvitation>, DomainError> {
        self.read_repo
            .find_by_organization_and_aggregate_id_status(organization_id, aggregate_id, status)
            .await
    }

    async fn find_by_status(
        &self,
        status: &InvitationStatus,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        self.read_repo.find_by_status(status).await
    }

    async fn find_expired(&self) -> Result<Vec<OrganizationInvitation>, DomainError> {
        self.read_repo.find_expired().await
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        self.read_repo.count_by_organization(organization_id).await
    }

    async fn count_pending_by_organization(
        &self,
        organization_id: &Uuid,
    ) -> Result<i64, DomainError> {
        self.read_repo
            .count_pending_by_organization(organization_id)
            .await
    }
}

#[async_trait]
impl OrganizationInvitationWriteRepository for OrganizationInvitationRepositoryImpl {
    async fn save(
        &self,
        invitation: &OrganizationInvitation,
    ) -> Result<OrganizationInvitation, DomainError> {
        self.write_repo.save(invitation).await
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }
}

impl OrganizationInvitationRepository for OrganizationInvitationRepositoryImpl {}
