use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{entity::*, error::DomainError, port::*, service::{member_service::MemberService, role_service::RoleService}, OrganizationService};

/// Domain service for organization invitation management
pub struct InvitationServiceImpl<IR, OS, RS, MS>
where
    IR: OrganizationInvitationRepository,
    OS: OrganizationService,
    RS: RoleService,
    MS: MemberService,
{
    invitation_repo: IR,
    organization_service: OS,
    role_service: RS,
    member_service: MS,
}

#[async_trait::async_trait]
pub trait InvitationService: Send + Sync {
    /**
     * Create an invitation to join an organization by email. used for non existing users
     * 
     * @param organization_id - The ID of the organization to invite the user to
     * @param email - The email of the user to invite
     * @param role_permissions - The roles to assign to the user
     * @param invited_by_user_id - The ID of the user who invited the user
     * @param message - The message to send to the user
     * @param expires_in_days - The number of days the invitation will expire
     */
    async fn create_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError>;

    /**
     * Create an invitation to join an organization by user. used for existing users
     * 
     * @param organization_id - The ID of the organization to invite the user to
     * @param user_id - The ID of the user to invite
     * @param role_permissions - The roles to assign to the user
     * @param invited_by_user_id - The ID of the user who invited the user
     * @param message - The message to send to the user
     * @param expires_in_days - The number of days the invitation will expire
     */
    async fn create_invitation_by_user(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError>;

    /**
     * Accept an invitation
     * 
     * @param token - The token of the invitation
     * @param user_id - The ID of the user accepting the invitation
     */
    async fn accept_invitation(
        &self,
        token: String,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError>;

    /**
     * Cancel an invitation
     * 
     * @param invitation_id - The ID of the invitation to cancel
     */
    async fn cancel_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<(), DomainError>;

    /**
     * Get an invitation by ID
     * 
     * @param invitation_id - The ID of the invitation to get
     */
    async fn get_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<OrganizationInvitation, DomainError>;

    /**
     * Get an invitation by organization and invited user
     * 
     * @param organization_id - The ID of the organization the invitation belongs to
     * @param invited_aggregate_id - The ID of the user the invitation is for
     */
    async fn get_invitation_by_organization_invited_aggregate_id(
        &self,
        organization_id: Uuid,
        invited_aggregate_id: &str,
    ) -> Result<OrganizationInvitation, DomainError>;

    /**
     * List invitations for an organization
     * 
     * @param organization_id - The ID of the organization to list the invitations for
     */
    async fn list_invitations(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError>;

    /**
     * Clean up expired invitations
     */
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError>;
}

impl<IR, OS, RS, MS> InvitationServiceImpl<IR, OS, RS, MS>
where
    IR: OrganizationInvitationRepository,
    OS: OrganizationService,
    RS: RoleService,
    MS: MemberService,
{
    /// Create a new invitation service
    pub fn new(invitation_repo: IR, organization_service: OS, role_service: RS, member_service: MS) -> Self {
        Self {
            invitation_repo,
            organization_service,
            role_service,
            member_service,
        }
    }
}

#[async_trait::async_trait]
impl<IR, OS, RS, MS> InvitationService for InvitationServiceImpl<IR, OS, RS, MS>
where
    IR: OrganizationInvitationRepository,
    OS: OrganizationService,
    RS: RoleService,
    MS: MemberService,
{
    /// Create an invitation to join an organization
    async fn create_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        _expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {    
        let invitation = OrganizationInvitation::new(organization_id, email, role_permissions, invited_by_user_id, message)?;
        Ok(self.invitation_repo.save(&invitation).await.map_err(|e| DomainError::Internal { message: e.to_string() })?)
    }

    /// Create an invitation to join an organization
    async fn create_invitation_by_user(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        let invitation = OrganizationInvitation::new(organization_id, user_id.to_string(), role_permissions, invited_by_user_id, message)?;
        Ok(self.invitation_repo.save(&invitation).await.map_err(|e| DomainError::Internal { message: e.to_string() })?)
    }

    /// Accept an invitation
    async fn accept_invitation(
        &self,
        token: String,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError> {
        let invitation = self.invitation_repo
        .find_by_token(&token)
        .await
        .map_err(|e| DomainError::Internal { message: e.to_string() })?;

        if invitation.is_none() {
            return Err(DomainError::entity_not_found("organization_invitation", &token));
        }

        let mut invitation = invitation.unwrap();

        if invitation.status != InvitationStatus::Pending {
            return Err(DomainError::business_rule_violation(format!("Invitation is not pending, but {:?}", invitation.status).as_str()));
        }

        if invitation.expires_at < Utc::now() {
            return Err(DomainError::business_rule_violation(format!("Invitation has expired at {}", invitation.expires_at).as_str()));
        }

        if invitation.accepted_at.is_some() {
            return Err(DomainError::business_rule_violation(format!("Invitation has already been accepted at {}", invitation.accepted_at.unwrap()).as_str()));
        }

        invitation.accept()?;
        
        self.invitation_repo.save(&invitation).await.map_err(|e| DomainError::Internal { message: e.to_string() })?;
        Ok(self.member_service
        .add_member(invitation.organization_id, user_id, invitation.role_permissions, Some(invitation.invited_by_user_id))
        .await.map_err(|e| DomainError::Internal { message: e.to_string() })?)
    }

    /// Cancel an invitation
    async fn cancel_invitation(
        &self,
        invitation_id: Uuid
    ) -> Result<(), DomainError> {
        let invitation = self.invitation_repo
        .find_by_id(&invitation_id)
        .await
        .map_err(|e| DomainError::Internal { message: e.to_string() })?;

        if invitation.is_none() {
            return Err(DomainError::entity_not_found("organization_invitation", &invitation_id.to_string()));
        }

        let mut invitation = invitation.unwrap();

        if invitation.status != InvitationStatus::Pending {
            return Err(DomainError::business_rule_violation(format!("Invitation is not pending, but {:?}", invitation.status).as_str()));
        }

        invitation.cancel()?;
        self.invitation_repo.save(&invitation).await.map_err(|e| DomainError::Internal { message: e.to_string() })?;
        Ok(())
    }

    /// Get invitation by ID
    async fn get_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<OrganizationInvitation, DomainError> {
        let invitation = self.invitation_repo
        .find_by_id(&invitation_id)
        .await
        .map_err(|e| DomainError::Internal { message: e.to_string() })?;

        if invitation.is_none() {
            return Err(DomainError::entity_not_found("organization_invitation", &invitation_id.to_string()));
        }

        Ok(invitation.unwrap())
    }

    /// Get invitation by organization and invited aggregate id
    async fn get_invitation_by_organization_invited_aggregate_id(
        &self,
        organization_id: Uuid,
        invited_aggregate_id: &str,
    ) -> Result<OrganizationInvitation, DomainError> {
        let invitation = self.invitation_repo
        .find_by_organization_and_aggregate_id_status(&organization_id, &invited_aggregate_id, &InvitationStatus::Pending)
        .await
        .map_err(|e| DomainError::Internal { message: e.to_string() })?;

        if invitation.is_none() {
            return Err(DomainError::entity_not_found("organization_invitation", &organization_id.to_string()));
        }

        Ok(invitation.unwrap())
    }

    /// List invitations for an organization
    async fn list_invitations(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        let invitations = self.invitation_repo
        .find_by_organization(&organization_id)
        .await
        .map_err(|e| DomainError::Internal { message: e.to_string() })?;

        Ok(invitations)
    }

    /// Clean up expired invitations
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError> {
        let expired_invitations = self.invitation_repo
        .find_expired()
        .await
        .map_err(|e| DomainError::Internal { message: e.to_string() })?;

        let count = expired_invitations.len();
        Ok(count as u32)
    }
}
