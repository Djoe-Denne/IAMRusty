use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{service::invitation_service::InvitationService, DomainError, entity::{RolePermission, OrganizationInvitation}};
use hive_events::{HiveDomainEvent, InvitationCreatedEvent, Role};
use rustycog_events::EventPublisher;

use crate::{
    dto::*,
    ApplicationError,
};

#[async_trait]
pub trait InvitationUseCase: Send + Sync {
    async fn create_invitation(
        &self,
        organization_id: Uuid,
        request: &CreateInvitationRequest,
        invited_by_user_id: Uuid,
    ) -> Result<InvitationResponse, ApplicationError>;

    async fn accept_invitation(&self, token: String, user_id: Uuid)
        -> Result<(), ApplicationError>;

    async fn cancel_invitation(&self, invitation_id: Uuid) -> Result<(), ApplicationError>;

    async fn get_invitation(&self, invitation_id: Uuid) -> Result<InvitationResponse, ApplicationError>;
}

pub struct InvitationUseCaseImpl {
    invitation_service: Arc<dyn InvitationService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl InvitationUseCaseImpl {
    pub fn new(
        invitation_service: Arc<dyn InvitationService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            invitation_service,
            event_publisher,
        }
    }

    fn invitation_to_response(&self, invitation: &OrganizationInvitation) -> InvitationResponse {
        InvitationResponse {
            id: invitation.id,
            organization_id: invitation.organization_id,
            organization_name: invitation.organization_name.clone().unwrap_or_default(),
            email: invitation.aggregate_id.clone(),
            roles: invitation.role_permissions.iter().map(|role| role.clone().into()).collect(),
            expires_at: invitation.expires_at,
            created_at: invitation.created_at,
            status: invitation.status.as_str().to_string(),
            message: invitation.message.clone(),
            invited_by_user_id: invitation.invited_by_user_id,
            token: invitation.token.clone(),
            accepted_at: invitation.accepted_at,
        }
    }
    /// Publish invitation created event
    async fn publish_invitation_created_event(
        &self,
        organization_id: Uuid,
        organization_name: &str,
        invitation_id: Uuid,
        email: &str,
        role_permissions: &Vec<RolePermission>,
        invited_by_user_id: Uuid,
        invitation_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::InvitationCreated(InvitationCreatedEvent::new(
            organization_id,
            organization_name.to_string(),
            invitation_id,
            email.to_string(),
            role_permissions.iter().map(|role| Role::new(role.permission.level.as_str().to_string(), role.resource.name.clone())).collect(),
            invited_by_user_id,
            invitation_token.to_string(),
            expires_at,
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }
}

#[async_trait]
impl InvitationUseCase for InvitationUseCaseImpl {
    async fn create_invitation(
        &self,
        organization_id: Uuid,
        request: &CreateInvitationRequest,
        invited_by_user_id: Uuid,
    ) -> Result<InvitationResponse, ApplicationError> {
        let role_permissions = request.roles.iter().map(|role| role.into()).collect();
        let invitation = self.invitation_service
        .create_invitation_by_email(organization_id.clone(), request.email.clone(), role_permissions, invited_by_user_id.clone(), request.message.clone(), None)
        .await
        .map_err(|e| ApplicationError::Domain(e))?;

        self.publish_invitation_created_event(
            organization_id,
            &invitation.organization_name.clone().unwrap_or_default(),
            invitation.id,
            &invitation.aggregate_id,
            &invitation.role_permissions,
            invited_by_user_id,
            &invitation.token,
            invitation.expires_at,
        ).await?;

        Ok(self.invitation_to_response(&invitation))
    }

    async fn accept_invitation(
        &self,
        token: String,
        user_id: Uuid,

    ) -> Result<(), ApplicationError> {
        let invitation = self.invitation_service.accept_invitation(token, user_id).await
        .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }

    async fn cancel_invitation(&self, invitation_id: Uuid) -> Result<(), ApplicationError> {
        let invitation = self.invitation_service.cancel_invitation(invitation_id).await
        .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }
    
    async fn get_invitation(&self, invitation_id: Uuid) -> Result<InvitationResponse, ApplicationError> {
        let invitation = self.invitation_service.get_invitation(invitation_id).await
        .map_err(|e| ApplicationError::Domain(e))?;

        Ok(self.invitation_to_response(&invitation))
    }
}
