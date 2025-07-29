use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

use crate::{
    entity::*,
    error::DomainError,
    port::*,
};

/// Domain service for organization invitation management
pub struct InvitationServiceImpl<IR, OR, RR, MR>
where
    IR: OrganizationInvitationRepository,
    OR: OrganizationRepository,
    RR: OrganizationRoleRepository,
    MR: OrganizationMemberRepository,
{
    invitation_repo: IR,
    organization_repo: OR,
    role_repo: RR,
    member_repo: MR,
}

#[async_trait::async_trait]
pub trait InvitationService {
    async fn create_invitation(&self, organization_id: Uuid, email: String, role_id: Uuid, invited_by_user_id: Uuid, message: Option<String>, expires_in_days: Option<i64>) -> Result<OrganizationInvitation, DomainError>;
    async fn accept_invitation(&self, token: String, user_id: Uuid) -> Result<OrganizationMember, DomainError>;
    async fn cancel_invitation(&self, invitation_id: Uuid, cancelled_by_user_id: Uuid) -> Result<(), DomainError>;
    async fn resend_invitation(&self, invitation_id: Uuid, resent_by_user_id: Uuid, expires_in_days: Option<i64>) -> Result<OrganizationInvitation, DomainError>;
    async fn get_invitation(&self, invitation_id: Uuid) -> Result<OrganizationInvitation, DomainError>;
    async fn get_invitation_by_token(&self, token: String) -> Result<OrganizationInvitation, DomainError>;
    async fn list_invitations(&self, organization_id: Uuid) -> Result<Vec<OrganizationInvitation>, DomainError>;
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError>;
    async fn check_invitation_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
}

impl<IR, OR, RR, MR> InvitationServiceImpl<IR, OR, RR, MR>
where
    IR: OrganizationInvitationRepository,
    OR: OrganizationRepository,
    RR: OrganizationRoleRepository,
    MR: OrganizationMemberRepository,
{
    /// Create a new invitation service
    pub fn new(
        invitation_repo: IR,
        organization_repo: OR,
        role_repo: RR,
        member_repo: MR,
    ) -> Self {
        Self {
            invitation_repo,
            organization_repo,
            role_repo,
            member_repo,
        }
    }

}

#[async_trait::async_trait]
impl<IR, OR, RR, MR> InvitationService for InvitationServiceImpl<IR, OR, RR, MR>
where
    IR: OrganizationInvitationRepository,
    OR: OrganizationRepository,
    RR: OrganizationRoleRepository,
    MR: OrganizationMemberRepository,
{
    /// Create an invitation to join an organization
    async fn create_invitation(
        &self,
        organization_id: Uuid,
        email: String,
        role_id: Uuid,
        invited_by_user_id: Uuid,
        message: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        // Validate organization exists
        let _organization = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Validate role exists and belongs to organization
        let role = self
            .role_repo
            .find_by_id(&role_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationRole", &role_id.to_string()))?;

        if role.organization_id != organization_id {
            return Err(DomainError::business_rule_violation(
                "Role does not belong to the specified organization",
            ));
        }

        // Business rule: Check permission to invite members
        self.check_invitation_permission(&organization_id, &invited_by_user_id).await?;

        // Business rule: Cannot invite organization owner 
        // Email/notification are hendle by Telegraph and trigger via SQS event in Usecase

        // Business rule: Check for existing pending invitation
        if let Some(_existing) = self
            .invitation_repo
            .find_pending_by_organization_and_email(&organization_id, &email)
            .await?
        {
            return Err(DomainError::resource_already_exists(
                "OrganizationInvitation",
                &format!("email={}, organization_id={}", email, organization_id),
            ));
        }

        // Business rule: Cannot invite to Owner role (only system can assign owner)
        if role.name == "Owner" {
            return Err(DomainError::business_rule_violation(
                "Cannot invite someone to Owner role",
            ));
        }

        // Calculate expiration date (default to 7 days)
        let expires_at = Utc::now() + Duration::days(expires_in_days.unwrap_or(7));

        // Create invitation
        let invitation = OrganizationInvitation::new_with_expiry(
            organization_id,
            email,
            role_id,
            invited_by_user_id,
            expires_at,
            message,
        )?;

        let saved_invitation = self.invitation_repo.save(&invitation).await?;

        Ok(saved_invitation)
    }

    /// Accept an invitation
    async fn accept_invitation(
        &self,
        token: String,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError> {
        // Find invitation by token
        let mut invitation = self
            .invitation_repo
            .find_by_token(&token)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationInvitation", &token))?;

        // Business rule: Check if invitation is still valid
        if !invitation.is_valid() {
            return Err(DomainError::business_rule_violation(
                "Invitation is no longer valid (already accepted, expired, or cancelled)",
            ));
        }

        // Business rule: Check if user is already a member
        if let Some(existing_member) = self
            .member_repo
            .find_by_organization_and_user(&invitation.organization_id, &user_id)
            .await?
        {
            if existing_member.is_active() {
                return Err(DomainError::business_rule_violation(
                    "User is already a member of this organization",
                ));
            }
            // If member exists but is not active, reactivate them
            let mut updated_member = existing_member;
            updated_member.reactivate()?;
            updated_member.update_role(invitation.role_id);
            let member = self.member_repo.save(&updated_member).await?;
            
            // Mark invitation as accepted
            invitation.accept();
            self.invitation_repo.save(&invitation).await?;
            
            return Ok(member);
        }

        // Create new member
        let member = OrganizationMember::new_from_invitation(
            invitation.organization_id,
            user_id,
            invitation.role_id,
            invitation.invited_by_user_id,
        );
        let saved_member = self.member_repo.save(&member).await?;

        // Mark invitation as accepted
        invitation.accept();
        self.invitation_repo.save(&invitation).await?;

        Ok(saved_member)
    }

    /// Cancel an invitation
    async fn cancel_invitation(
        &self,
        invitation_id: Uuid,
        cancelled_by_user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Find invitation
        let mut invitation = self
            .invitation_repo
            .find_by_id(&invitation_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationInvitation", &invitation_id.to_string()))?;

        // Business rule: Check permission to cancel invitation
        self.check_invitation_permission(&invitation.organization_id, &cancelled_by_user_id).await?;

        // Business rule: Can only cancel valid invitations
        if !invitation.is_valid() {
            return Err(DomainError::business_rule_violation(
                "Can only cancel valid invitations",
            ));
        }

        // Cancel the invitation
        invitation.cancel();
        self.invitation_repo.save(&invitation).await?;

        Ok(())
    }

    /// Resend an invitation (creates new token and extends expiry)
    async fn resend_invitation(
        &self,
        invitation_id: Uuid,
        resent_by_user_id: Uuid,
        expires_in_days: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        // Find invitation
        let mut invitation = self
            .invitation_repo
            .find_by_id(&invitation_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationInvitation", &invitation_id.to_string()))?;

        // Business rule: Check permission to resend invitation
        self.check_invitation_permission(&invitation.organization_id, &resent_by_user_id).await?;

        // Business rule: Can only resend valid invitations
        if !invitation.is_valid() {
            return Err(DomainError::business_rule_violation(
                "Can only resend valid invitations",
            ));
        }

        // Generate new token and extend expiry (recreate invitation with new expiry)
        let new_expires_at = Utc::now() + Duration::days(expires_in_days.unwrap_or(7));
        invitation.expires_at = new_expires_at;
        invitation.token = format!("{}{}", 
            uuid::Uuid::new_v4().simple().to_string(),
            uuid::Uuid::new_v4().simple().to_string()
        )[..64].to_string();

        let updated_invitation = self.invitation_repo.save(&invitation).await?;

        Ok(updated_invitation)
    }

    /// Get invitation by ID
    async fn get_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<OrganizationInvitation, DomainError> {
        self.invitation_repo
            .find_by_id(&invitation_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationInvitation", &invitation_id.to_string()))
    }

    /// Get invitation by token (for public access)
    async fn get_invitation_by_token(
        &self,
        token: String,
    ) -> Result<OrganizationInvitation, DomainError> {
        self.invitation_repo
            .find_by_token(&token)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationInvitation", &token))
    }

    /// List invitations for an organization
    async fn list_invitations(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationInvitation>, DomainError> {
        // Validate organization exists
        self.organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        self.invitation_repo
            .find_by_organization(&organization_id)
            .await
    }

    /// Clean up expired invitations
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError> {
        let expired_invitations = self.invitation_repo.find_expired().await?;
        let count = expired_invitations.len() as u32;

        for mut invitation in expired_invitations {
            invitation.mark_expired();
            self.invitation_repo.save(&invitation).await?;
        }

        Ok(count)
    }

    /// Check if user has permission to manage invitations
    async fn check_invitation_permission(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<(), DomainError> {
        // Find the organization
        let organization = self
            .organization_repo
            .find_by_id(organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Owner always has permission
        if organization.is_owned_by(user_id) {
            return Ok(());
        }

        // Check if user is a member with admin role
        let member = self
            .member_repo
            .find_by_organization_and_user(organization_id, user_id)
            .await?;

        if let Some(member) = member {
            if member.is_active() {
                // Get the role and check if it has admin permissions
                let role = self
                    .role_repo
                    .find_by_id(&member.role_id)
                    .await?
                    .ok_or_else(|| DomainError::entity_not_found("OrganizationRole", &member.role_id.to_string()))?;

                if role.is_admin() {
                    return Ok(());
                }
            }
        }

        Err(DomainError::permission_denied(
            "User does not have permission to manage invitations in this organization",
        ))
    }
} 