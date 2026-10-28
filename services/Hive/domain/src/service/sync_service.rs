use std::sync::Arc;
use uuid::Uuid;

use crate::{
    entity::{ExternalLink, Organization, SyncJob, SyncJobType},
    port::{
        service::{ExternalMember, ExternalOrganizationInfo, ExternalProviderClient},
        ExternalLinkRepository, OrganizationRepository, SyncJobRepository,
    },
    service::{invitation_service::InvitationService, organization_service::OrganizationService},
};
use rustycog::core::error::DomainError;
use rustycog::permission::{PrincipalPolicy, PrincipalRef};

/// Domain service for sync job management
pub struct SyncServiceImpl<SR, LR, OR, OS, IS, PC>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
    OS: OrganizationService,
    IS: InvitationService,
    PC: ExternalProviderClient,
{
    sync_job_repo: Arc<SR>,
    external_link_repo: Arc<LR>,
    organization_repo: Arc<OR>,
    organization_service: Arc<OS>,
    invitation_service: Arc<IS>,
    provider_client: Arc<PC>,
    principal_policy: Arc<PrincipalPolicy>,
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
    /// Start a persisted sync job for an external link.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the link is missing, sync is disabled, a job is already running, or persistence fails.
    async fn start_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError>;

    /// Build a sync job without persisting it.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the link is missing, sync is disabled, or a job is already running.
    async fn prepare_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError>;

    /// Execute sync for organization info.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the job, link, or organization is missing, or the provider call fails.
    async fn sync_organization_info(&self, sync_job_id: Uuid) -> Result<Organization, DomainError>;

    /// Execute sync for members.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the job, link, or organization is missing, or the provider call fails.
    async fn sync_members(
        &self,
        sync_job_id: Uuid,
        auto_invite: bool,
    ) -> Result<SyncResult, DomainError>;
}

impl<SR, LR, OR, OS, IS, PC> SyncServiceImpl<SR, LR, OR, OS, IS, PC>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
    OS: OrganizationService,
    IS: InvitationService,
    PC: ExternalProviderClient,
{
    /// Create a new sync service
    pub const fn new(
        sync_job_repo: Arc<SR>,
        external_link_repo: Arc<LR>,
        organization_repo: Arc<OR>,
        organization_service: Arc<OS>,
        invitation_service: Arc<IS>,
        provider_client: Arc<PC>,
        principal_policy: Arc<PrincipalPolicy>,
    ) -> Self {
        Self {
            sync_job_repo,
            external_link_repo,
            organization_repo,
            organization_service,
            invitation_service,
            provider_client,
            principal_policy,
        }
    }

    /// Update organization info from external provider data
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the organization update fails.
    async fn update_organization_from_external(
        &self,
        organization_id: Uuid,
        external_org_info: &ExternalOrganizationInfo,
        _requesting_user_id: Uuid,
    ) -> Result<Organization, DomainError> {
        // Update organization with external info
        let updated_org = self
            .organization_service
            .update_organization(
                organization_id,
                Some(
                    external_org_info
                        .display_name
                        .clone()
                        .unwrap_or_else(|| external_org_info.name.clone()),
                ),
                external_org_info.description.clone(),
                external_org_info.avatar_url.clone(),
                None, // Don't override settings
            )
            .await?;

        Ok(updated_org)
    }

    async fn process_external_member_for_sync(
        &self,
        external_member: ExternalMember,
        external_link: &ExternalLink,
        organization: &Organization,
        auto_invite: bool,
        invitation_message: Option<&str>,
        result: &mut SyncResult,
    ) {
        if !external_member.is_active {
            return;
        }

        let invite_identifier = if let Some(email) = &external_member.email {
            email.clone()
        } else {
            result.errors.push(format!(
                "External member {} has no email address, skipping",
                external_member.username
            ));
            return;
        };

        let existing_invitation = self
            .invitation_service
            .get_invitation_by_organization_invited_aggregate_id(
                external_link.organization_id,
                &invite_identifier,
            )
            .await;

        if existing_invitation.is_ok() {
            return;
        }

        if auto_invite {
            let role_permissions = external_member.roles.clone();

            if role_permissions.is_empty() {
                result.errors.push(format!(
                    "External member {} has no role permissions, skipping",
                    external_member.username
                ));
                return;
            }

            match self
                .invitation_service
                .create_invitation_by_email(
                    external_link.organization_id,
                    invite_identifier,
                    role_permissions,
                    organization.owner_user_id,
                    invitation_message.map(str::to_owned),
                    None,
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
}

#[async_trait::async_trait]
impl<SR, LR, OR, OS, IS, PC> SyncService for SyncServiceImpl<SR, LR, OR, OS, IS, PC>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
    OS: OrganizationService,
    IS: InvitationService,
    PC: ExternalProviderClient,
{
    /// Build a sync job without persisting it.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the link is missing, sync is disabled, or a job is already running.
    async fn prepare_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        _requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError> {
        let external_link = self
            .external_link_repo
            .find_by_id(&external_link_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found("ExternalLink", &external_link_id.to_string())
            })?;

        if !external_link.is_sync_enabled() {
            return Err(DomainError::business_rule_violation(
                "Sync is not enabled for this external link",
            ));
        }

        let running_jobs = self
            .sync_job_repo
            .find_running_by_external_link(&external_link_id)
            .await?;

        if !running_jobs.is_empty() {
            return Err(DomainError::business_rule_violation(
                "A sync job is already running for this external link",
            ));
        }

        Ok(SyncJob::new(external_link_id, job_type, None))
    }

    /// Start a persisted sync job for an external link.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the link is missing, sync is disabled, a job is already running, or persistence fails.
    async fn start_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError> {
        let sync_job = self
            .prepare_sync_job(external_link_id, job_type, requested_by_user_id)
            .await?;
        self.sync_job_repo.save(&sync_job).await
    }

    /// Execute sync for organization info
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the job, link, or organization is missing, or the provider call fails.
    async fn sync_organization_info(&self, sync_job_id: Uuid) -> Result<Organization, DomainError> {
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
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "ExternalLink",
                    &sync_job.organization_external_link_id.to_string(),
                )
            })?;

        // Get organization info from external provider
        let provider_source = external_link
            .provider_source
            .clone()
            .ok_or_else(|| DomainError::internal_error("external link missing provider_source"))?;
        let external_org_info = self
            .provider_client
            .get_organization_info(&provider_source, &external_link.provider_config)
            .await?;

        // Update organization with external info
        // Use organization owner as the requesting user for updates
        let organization = self
            .organization_repo
            .find_by_id(&external_link.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "Organization",
                    &external_link.organization_id.to_string(),
                )
            })?;

        let updated_org = self
            .update_organization_from_external(
                external_link.organization_id,
                &external_org_info,
                organization.owner_user_id,
            )
            .await?;

        Ok(updated_org)
    }

    /// Execute sync for members
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the job, link, or organization is missing, or the provider call fails.
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
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "ExternalLink",
                    &sync_job.organization_external_link_id.to_string(),
                )
            })?;

        // Re-read authoritative owner provenance even for historical jobs.
        // Reject UUID-only invitations before contacting the external provider.
        let organization = self
            .organization_repo
            .find_by_id(&external_link.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "Organization",
                    &external_link.organization_id.to_string(),
                )
            })?;
        require_auto_invite_owner(&self.principal_policy, &organization, auto_invite)?;

        // Get members from external provider
        let provider_source = external_link
            .provider_source
            .clone()
            .ok_or_else(|| DomainError::internal_error("external link missing provider_source"))?;
        let external_members = self
            .provider_client
            .get_members(&provider_source, &external_link.provider_config)
            .await?;

        let mut result = SyncResult {
            members_found: u32::try_from(external_members.len()).unwrap_or(u32::MAX),
            members_added: 0,
            members_invited: 0,
            errors: Vec::new(),
        };

        // Create invitation
        let invitation_message = Some(format!(
            "You have been invited to join {} organization based on your membership in the connected {:?} organization.",
            organization.name,
            provider_source
        ));

        for external_member in external_members {
            Box::pin(self.process_external_member_for_sync(
                external_member,
                &external_link,
                &organization,
                auto_invite,
                invitation_message.as_deref(),
                &mut result,
            ))
            .await;
        }

        Ok(result)
    }
}

fn require_auto_invite_owner(
    policy: &PrincipalPolicy,
    organization: &Organization,
    auto_invite: bool,
) -> Result<(), DomainError> {
    if !auto_invite {
        return Ok(());
    }
    let incompatible = || DomainError::permission_denied("PRINCIPAL_INCOMPATIBLE");
    let issuer = organization
        .owner_issuer
        .as_deref()
        .ok_or_else(incompatible)?;
    let owner =
        PrincipalRef::new(issuer, organization.owner_user_id).map_err(|_| incompatible())?;
    policy
        .platform_user_id(&owner)
        .map_err(|_| incompatible())?;
    Ok(())
}

#[cfg(test)]
#[path = "sync_service_principal_tests.rs"]
mod principal_tests;
