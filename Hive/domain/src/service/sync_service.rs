use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    entity::*, 
    error::DomainError, 
    port::{repository::*, service::*},
    service::{
        role_service::RoleService,
        organization_service::OrganizationService,
        invitation_service::InvitationService,
    },
};

/// Domain service for sync job management
pub struct SyncServiceImpl<SR, LR, OR, RS, OS, IS, PC>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
    RS: RoleService,
    OS: OrganizationService,
    IS: InvitationService,
    PC: ExternalProviderClient,
{
    sync_job_repo: SR,
    external_link_repo: LR,
    organization_repo: OR,
    role_service: RS,
    organization_service: OS,
    invitation_service: IS,
    provider_client: PC,
}

/// Result of a member sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub members_found: u32,
    pub members_added: u32,
    pub members_invited: u32,
    pub errors: Vec<String>,
}

#[async_trait::async_trait]
pub trait SyncService: Send + Sync {
    async fn start_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError>;

    /**
     * Execute sync for organization info
     * 
     * @param sync_job_id - The ID of the sync job
     */
    async fn sync_organization_info(
        &self,
        sync_job_id: Uuid, 
    ) -> Result<Organization, DomainError>;

    /**
     * Execute sync for members
     * 
     * @param sync_job_id - The ID of the sync job
     * @param auto_invite - Whether to automatically invite new members found
     */
    async fn sync_members(
        &self,
        sync_job_id: Uuid,
        auto_invite: bool,
    ) -> Result<SyncResult, DomainError>;
}

impl<SR, LR, OR, RS, OS, IS, PC> SyncServiceImpl<SR, LR, OR, RS, OS, IS, PC>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
    RS: RoleService,
    OS: OrganizationService,
    IS: InvitationService,
    PC: ExternalProviderClient,
{
    /// Create a new sync service
    pub fn new(
        sync_job_repo: SR, 
        external_link_repo: LR, 
        organization_repo: OR,
        role_service: RS,
        organization_service: OS,
        invitation_service: IS,
        provider_client: PC,
    ) -> Self {
        Self { sync_job_repo, external_link_repo, organization_repo, role_service, organization_service, invitation_service, provider_client }
    }

    /// Update organization info from external provider data
    async fn update_organization_from_external(
        &self,
        organization_id: Uuid,
        external_org_info: &ExternalOrganizationInfo,
        requesting_user_id: Uuid,
    ) -> Result<Organization, DomainError> {
        // Update organization with external info
        let updated_org = self
            .organization_service
            .update_organization(
                organization_id.clone(),
                Some(external_org_info.display_name.clone().unwrap_or(external_org_info.name.clone())),
                external_org_info.description.clone(),
                external_org_info.avatar_url.clone(),
                None, // Don't override settings
                requesting_user_id.clone(),
            )
            .await?;

        Ok(updated_org)
    }
}


#[async_trait::async_trait]
impl<SR, LR, OR, RS, OS, IS, PC> SyncService for SyncServiceImpl<SR, LR, OR, RS, OS, IS, PC>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
    RS: RoleService,
    OS: OrganizationService,
    IS: InvitationService,
    PC: ExternalProviderClient,
{
    /// Start a new sync job
    async fn start_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError> {
        // Validate external link exists
        let external_link = self
            .external_link_repo
            .find_by_id(&external_link_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found("ExternalLink", &external_link_id.to_string())
            })?;

        // Business rule: Check permission to start sync jobs
        self.role_service.check_admin_permission(&external_link.organization_id, &requested_by_user_id, "organization")
            .await?;

        // Business rule: Sync must be enabled for the external link
        if !external_link.is_sync_enabled() {
            return Err(DomainError::business_rule_violation(
                "Sync is not enabled for this external link",
            ));
        }

        // Business rule: Check if there's already a running job for this external link
        let running_jobs = self
            .sync_job_repo
            .find_running_by_external_link(&external_link_id)
            .await?;

        if !running_jobs.is_empty() {
            return Err(DomainError::business_rule_violation(
                "A sync job is already running for this external link",
            ));
        }

        // Create new sync job
        let sync_job = SyncJob::new(external_link_id, job_type, None);
        let saved_job = self.sync_job_repo.save(&sync_job).await?;

        Ok(saved_job)
    }

    /// Execute sync for organization info
    async fn sync_organization_info(
        &self,
        sync_job_id: Uuid,
    ) -> Result<Organization, DomainError> {
        // Find the sync job
        let sync_job = self
            .sync_job_repo
            .find_by_id(&sync_job_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("SyncJob", &sync_job_id.to_string()))?;

        // Find the external link
        let external_link = self
            .external_link_repo
            .find_by_id(&sync_job.organization_external_link_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExternalLink", &sync_job.organization_external_link_id.to_string()))?;

        // Get organization info from external provider
        let external_org_info = self.provider_client
            .get_organization_info(&external_link.provider_source.clone().unwrap(), &external_link.provider_config)
            .await?;

        // Update organization with external info
        // Use organization owner as the requesting user for updates
        let organization = self
            .organization_repo
            .find_by_id(&external_link.organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &external_link.organization_id.to_string()))?;

        let updated_org = self
            .update_organization_from_external(
                external_link.organization_id.clone(),
                &external_org_info,
                organization.owner_user_id.clone(),
            )
            .await?;

        Ok(updated_org)
    }

    /// Execute sync for members
    async fn sync_members(
        &self,
        sync_job_id: Uuid,
        auto_invite: bool,
    ) -> Result<SyncResult, DomainError> {
        // Find the sync job
        let sync_job = self
            .sync_job_repo
            .find_by_id(&sync_job_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("SyncJob", &sync_job_id.to_string()))?;

        // Find the external link
        let external_link = self
            .external_link_repo
            .find_by_id(&sync_job.organization_external_link_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExternalLink", &sync_job.organization_external_link_id.to_string()))?;
        

        // Get members from external provider
        let external_members = self.provider_client
            .get_members(&external_link.provider_source.clone().unwrap(), &external_link.provider_config)
            .await?;

        let mut result = SyncResult {
            members_found: external_members.len() as u32,
            members_added: 0,
            members_invited: 0,
            errors: Vec::new(),
        };

        // Get organization for owner information
        let organization = self
            .organization_repo
            .find_by_id(&external_link.organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &external_link.organization_id.to_string()))?;

        // Create invitation
        let invitation_message = Some(format!(
            "You have been invited to join {} organization based on your membership in the connected {:?} organization.",
            organization.name,
            external_link.provider_source.clone().unwrap()
        ));

        // Process each external member
        for external_member in external_members {
            if !external_member.is_active {
                continue; // Skip inactive members
            }

            // For now, we'll invite by email if available, otherwise skip
            let invite_identifier = match &external_member.email {
                Some(email) => email.clone(),
                None => {
                    result.errors.push(format!(
                        "External member {} has no email address, skipping",
                        external_member.username
                    ));
                    continue;
                }
            };

            // Check if user is already a member
            // Note: This is a simplified check. In a real implementation, you might want to
            // maintain a mapping of external IDs to internal user IDs
            let existing_invitation = self
                .invitation_service
                .get_invitation_by_organization_invited_aggregate_id(
                    external_link.organization_id,
                    &invite_identifier,
                )
                .await;

            if existing_invitation.is_ok() {
                continue; // Already invited
            }

            if auto_invite {
                // Get role permissions for this external member
                let role_permissions = external_member.roles.clone();

                // Skip if no role permissions are available
                if role_permissions.is_empty() {
                    result.errors.push(format!(
                        "External member {} has no role permissions, skipping",
                        external_member.username
                    ));
                    continue;
                }

                match self
                    .invitation_service
                    .create_invitation_by_email(
                        external_link.organization_id,
                        invite_identifier,
                        role_permissions,
                        organization.owner_user_id, // Invitations are sent by the organization owner
                        invitation_message.clone(),
                        None, // Use default expiry
                    )
                    .await
                {
                    Ok(_) => result.members_invited += 1,
                    Err(e) => result.errors.push(format!(
                        "Failed to invite {}: {}",
                        external_member.username, e
                    )),
                }
            }
        }

        Ok(result)
    }
}
