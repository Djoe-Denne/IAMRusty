use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    entity::{InvitationStatus, OrganizationInvitation, OrganizationMember, RolePermission},
    port::OrganizationInvitationRepository,
    service::member_service::MemberService,
    OrganizationService,
};
use rustycog::core::error::DomainError;

/// Domain service for organization invitation management
pub struct InvitationServiceImpl<IR, OS, MS>
where
    IR: OrganizationInvitationRepository,
    OS: OrganizationService,
    MS: MemberService,
{
    invitation_repo: Arc<IR>,
    organization_service: Arc<OS>,
    member_service: Arc<MS>,
}

#[async_trait::async_trait]
pub trait InvitationService: Send + Sync {
    /// Create an invitation to join an organization by email for a user that does not exist yet.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization is missing, the invitation is invalid, or persistence fails.
    async fn create_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError>;

    /// Build an invitation by email without persisting it.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization is missing or the invitation data is invalid.
    async fn prepare_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError>;

    /// Create an invitation to join an organization for an existing user.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization is missing, the invitation is invalid, or persistence fails.
    async fn create_invitation_by_user(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError>;

    /// Accept an invitation.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing, not pending, expired, already accepted, or persistence fails.
    async fn accept_invitation(
        &self,
        token: String,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError>;

    /// Cancel an invitation.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing, not pending, or persistence fails.
    async fn cancel_invitation(&self, invitation_id: Uuid) -> Result<(), DomainError>;

    /// Get an invitation by ID.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing or persistence fails.
    async fn get_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<OrganizationInvitation, DomainError>;

    /// Get a pending invitation by organization and invited aggregate id.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing or persistence fails.
    async fn get_invitation_by_organization_invited_aggregate_id(
        &self,
        organization_id: Uuid,
        invited_aggregate_id: &str,
    ) -> Result<OrganizationInvitation, DomainError>;

    /// List invitations for an organization.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn list_invitations(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError>;

    /// Count expired invitations that can be cleaned up.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError>;
}

impl<IR, OS, MS> InvitationServiceImpl<IR, OS, MS>
where
    IR: OrganizationInvitationRepository,
    OS: OrganizationService,
    MS: MemberService,
{
    /// Create a new invitation service
    pub const fn new(
        invitation_repo: Arc<IR>,
        organization_service: Arc<OS>,
        member_service: Arc<MS>,
    ) -> Self {
        Self {
            invitation_repo,
            organization_service,
            member_service,
        }
    }
}

#[async_trait::async_trait]
impl<IR, OS, MS> InvitationService for InvitationServiceImpl<IR, OS, MS>
where
    IR: OrganizationInvitationRepository,
    OS: OrganizationService,
    MS: MemberService,
{
    /// Create an invitation to join an organization
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization is missing or the invitation data is invalid.
    async fn prepare_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        _expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;
        let mut invitation = OrganizationInvitation::new(
            organization_id,
            email,
            role_permissions,
            invited_by_user_id,
            message,
        )?;
        invitation.update_organization_name(&organization.name);
        Ok(invitation)
    }

    /// Persist an invitation created from an email address.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization is missing, the invitation is invalid, or persistence fails.
    async fn create_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        let invitation = self
            .prepare_invitation_by_email(
                organization_id,
                email,
                role_permissions,
                invited_by_user_id,
                message,
                expires_in_days,
            )
            .await?;
        self.invitation_repo
            .save(&invitation)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })
    }

    /// Create an invitation to join an organization
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization is missing, the invitation is invalid, or persistence fails.
    async fn create_invitation_by_user(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        _expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;
        let invitation = OrganizationInvitation::new(
            organization_id,
            user_id.to_string(),
            role_permissions,
            invited_by_user_id,
            message,
        )?;
        let mut saved_invitation =
            self.invitation_repo
                .save(&invitation)
                .await
                .map_err(|e| DomainError::Internal {
                    message: e.to_string(),
                })?;
        saved_invitation.update_organization_name(&organization.name);
        Ok(saved_invitation)
    }

    /// Accept an invitation
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing, not pending, expired, already accepted, or persistence fails.
    async fn accept_invitation(
        &self,
        token: String,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError> {
        let invitation = self
            .invitation_repo
            .find_by_token(&token)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        let mut invitation = invitation
            .ok_or_else(|| DomainError::entity_not_found("organization_invitation", &token))?;

        if invitation.status != InvitationStatus::Pending {
            return Err(DomainError::business_rule_violation(
                format!("Invitation is not pending, but {:?}", invitation.status).as_str(),
            ));
        }

        if invitation.expires_at < Utc::now() {
            return Err(DomainError::business_rule_violation(
                format!("Invitation has expired at {}", invitation.expires_at).as_str(),
            ));
        }

        if let Some(accepted_at) = invitation.accepted_at {
            return Err(DomainError::business_rule_violation(
                format!("Invitation has already been accepted at {accepted_at}").as_str(),
            ));
        }

        invitation.accept()?;

        self.invitation_repo
            .save(&invitation)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;
        Ok(self
            .member_service
            .add_member(
                invitation.organization_id,
                user_id,
                invitation.role_permissions,
                Some(invitation.invited_by_user_id),
            )
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?)
    }

    /// Cancel an invitation
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing, not pending, or persistence fails.
    async fn cancel_invitation(&self, invitation_id: Uuid) -> Result<(), DomainError> {
        let invitation = self
            .invitation_repo
            .find_by_id(&invitation_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        let mut invitation = invitation.ok_or_else(|| {
            DomainError::entity_not_found("organization_invitation", &invitation_id.to_string())
        })?;

        if invitation.status != InvitationStatus::Pending {
            return Err(DomainError::business_rule_violation(
                format!("Invitation is not pending, but {:?}", invitation.status).as_str(),
            ));
        }

        invitation.cancel()?;
        self.invitation_repo
            .save(&invitation)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;
        Ok(())
    }

    /// Get invitation by ID
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing or persistence fails.
    async fn get_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<OrganizationInvitation, DomainError> {
        let invitation = self
            .invitation_repo
            .find_by_id(&invitation_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        let mut invitation = invitation.ok_or_else(|| {
            DomainError::entity_not_found("organization_invitation", &invitation_id.to_string())
        })?;

        let organization = self
            .organization_service
            .get_organization(&invitation.organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;
        invitation.update_organization_name(&organization.name);
        Ok(invitation)
    }

    /// Get invitation by organization and invited aggregate id
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the invitation is missing or persistence fails.
    async fn get_invitation_by_organization_invited_aggregate_id(
        &self,
        organization_id: Uuid,
        invited_aggregate_id: &str,
    ) -> Result<OrganizationInvitation, DomainError> {
        let invitation = self
            .invitation_repo
            .find_by_organization_and_aggregate_id_status(
                &organization_id,
                invited_aggregate_id,
                &InvitationStatus::Pending,
            )
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        invitation.ok_or_else(|| {
            DomainError::entity_not_found("organization_invitation", &organization_id.to_string())
        })
    }

    /// List invitations for an organization
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn list_invitations(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        let invitations = self
            .invitation_repo
            .find_by_organization(&organization_id)
            .await
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        Ok(invitations)
    }

    /// Clean up expired invitations
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persistence fails.
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError> {
        let expired_invitations =
            self.invitation_repo
                .find_expired()
                .await
                .map_err(|e| DomainError::Internal {
                    message: e.to_string(),
                })?;

        let count = u32::try_from(expired_invitations.len()).unwrap_or(u32::MAX);
        Ok(count)
    }
}
