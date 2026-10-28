//! Native policy/order regressions. These private probes exercise the real sync
//! service; no database, provider transport, or testcontainer is started.
use super::*;
use crate::entity::{OrganizationInvitation, OrganizationMember, RolePermission, SyncJobStatus};
use crate::port::{
    ExternalLinkReadRepository, ExternalLinkWriteRepository, OrganizationReadRepository,
    OrganizationWriteRepository, SyncJobReadRepository, SyncJobWriteRepository,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

const PLATFORM: &str = "https://platform.example/iam";
const FOREIGN: &str = "https://foreign.example/iam";

struct Probe {
    organization: Mutex<Organization>,
    link: ExternalLink,
    job: SyncJob,
    saved_job: Mutex<Option<SyncJob>>,
    provider_calls: AtomicUsize,
    job_writes: AtomicUsize,
    organization_writes: AtomicUsize,
    link_writes: AtomicUsize,
    invitation_reads: AtomicUsize,
    invitation_creates: AtomicUsize,
}

type Service = SyncServiceImpl<Probe, Probe, Probe, Probe, Probe, Probe>;

fn fixture(issuer: Option<&str>) -> (Service, Arc<Probe>) {
    let mut organization =
        Organization::new("Sync".into(), "sync".into(), None, Uuid::new_v4()).unwrap();
    organization.owner_issuer = issuer.map(str::to_owned);
    let mut link = ExternalLink::new(
        organization.id,
        None,
        Uuid::new_v4(),
        Some("fixture".into()),
        serde_json::json!({"organization": "fixture"}),
        None,
    )
    .unwrap();
    link.sync_enabled = true;
    let job = SyncJob::new(link.id, SyncJobType::MembersOnly, None);
    let probe = Arc::new(Probe {
        organization: Mutex::new(organization),
        link,
        job,
        saved_job: Mutex::new(None),
        provider_calls: AtomicUsize::new(0),
        job_writes: AtomicUsize::new(0),
        organization_writes: AtomicUsize::new(0),
        link_writes: AtomicUsize::new(0),
        invitation_reads: AtomicUsize::new(0),
        invitation_creates: AtomicUsize::new(0),
    });
    let service = SyncServiceImpl::new(
        probe.clone(),
        probe.clone(),
        probe.clone(),
        probe.clone(),
        probe.clone(),
        probe.clone(),
        Arc::new(PrincipalPolicy::new(PLATFORM, false).unwrap()),
    );
    (service, probe)
}

#[test]
fn auto_invite_owner_requires_exact_platform_provenance() {
    let (service, probe) = fixture(Some(PLATFORM));
    let mut organization = probe.organization.lock().unwrap().clone();
    assert!(require_auto_invite_owner(&service.principal_policy, &organization, true).is_ok());
    for issuer in [
        None,
        Some(FOREIGN),
        Some("https://platform.example/iam/"),
        Some("iamrusty"),
        Some(""),
    ] {
        organization.owner_issuer = issuer.map(str::to_owned);
        let error =
            require_auto_invite_owner(&service.principal_policy, &organization, true).unwrap_err();
        assert!(
            matches!(error, DomainError::PermissionDenied { message } if message == "PRINCIPAL_INCOMPATIBLE")
        );
    }
    // Historical provenance is only accepted in its explicitly enabled window.
    organization.owner_issuer = Some("iamrusty".into());
    assert!(require_auto_invite_owner(
        &PrincipalPolicy::new(PLATFORM, true).unwrap(),
        &organization,
        true
    )
    .is_ok());
}

#[tokio::test]
async fn historical_auto_invite_job_rechecks_owner_before_provider() {
    let (service, probe) = fixture(Some(PLATFORM));
    let result = service.sync_members(probe.job.id, true).await.unwrap();
    assert_eq!(result.members_found, 1);
    assert_eq!(result.members_invited, 1);
    assert!(result.errors.is_empty());
    assert_eq!(probe.provider_calls.load(Ordering::SeqCst), 1);
    assert_eq!(probe.invitation_reads.load(Ordering::SeqCst), 1);
    assert_eq!(probe.invitation_creates.load(Ordering::SeqCst), 1);
    for issuer in [Some(FOREIGN), None] {
        // Same persisted job, changed current writer provenance: never trust
        // the owner at scheduling time or a same-UUID requesting user.
        probe.organization.lock().unwrap().owner_issuer = issuer.map(str::to_owned);
        let error = service.sync_members(probe.job.id, true).await.unwrap_err();
        assert!(
            matches!(error, DomainError::PermissionDenied { message } if message == "PRINCIPAL_INCOMPATIBLE")
        );
        assert_eq!(
            probe.provider_calls.load(Ordering::SeqCst),
            1,
            "denial must precede provider I/O"
        );
        assert_eq!(
            probe.job_writes.load(Ordering::SeqCst),
            0,
            "denial must not mutate the historical job"
        );
        assert_eq!(probe.organization_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.link_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.invitation_reads.load(Ordering::SeqCst), 1);
        assert_eq!(
            probe.invitation_creates.load(Ordering::SeqCst),
            1,
            "denial must not create another invitation"
        );
        assert!(probe.saved_job.lock().unwrap().is_none());
    }
}

#[tokio::test]
async fn sync_without_auto_invite_accepts_foreign_or_unresolved_owner() {
    for issuer in [Some(FOREIGN), None] {
        let (service, probe) = fixture(issuer);
        let result = service.sync_members(probe.job.id, false).await.unwrap();
        assert_eq!(result.members_found, 1);
        assert_eq!(result.members_invited, 0);
        assert!(result.errors.is_empty());
        assert_eq!(probe.provider_calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.job_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.organization_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.link_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.invitation_creates.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn non_inviting_job_creation_does_not_require_platform_owner() {
    for issuer in [Some(FOREIGN), None] {
        let (service, probe) = fixture(issuer);
        let job = service
            .start_sync_job(probe.link.id, SyncJobType::MembersOnly, Uuid::new_v4())
            .await
            .expect("scheduling a job does not express an invitation intent");
        assert_eq!(job.organization_external_link_id, probe.link.id);
        assert_eq!(probe.job_writes.load(Ordering::SeqCst), 1);
        assert_eq!(probe.provider_calls.load(Ordering::SeqCst), 0);
        let error = service.sync_members(job.id, true).await.unwrap_err();
        assert!(
            matches!(error, DomainError::PermissionDenied { message } if message == "PRINCIPAL_INCOMPATIBLE")
        );
        assert_eq!(
            probe.job_writes.load(Ordering::SeqCst),
            1,
            "execution denial must not change the previously admitted job"
        );
        assert_eq!(probe.provider_calls.load(Ordering::SeqCst), 0);
        assert_eq!(probe.organization_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.link_writes.load(Ordering::SeqCst), 0);
        assert_eq!(probe.invitation_reads.load(Ordering::SeqCst), 0);
        assert_eq!(probe.invitation_creates.load(Ordering::SeqCst), 0);
        assert_eq!(probe.saved_job.lock().unwrap().as_ref(), Some(&job));
    }
}

#[async_trait]
impl SyncJobReadRepository for Probe {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<SyncJob>, DomainError> {
        if let Some(job) = self
            .saved_job
            .lock()
            .unwrap()
            .as_ref()
            .filter(|job| job.id == *id)
        {
            return Ok(Some(job.clone()));
        }
        Ok((*id == self.job.id).then(|| self.job.clone()))
    }
    async fn find_running_by_external_link(&self, _: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        Ok(vec![])
    }
    async fn find_by_external_link(&self, _: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        unreachable!()
    }
    async fn find_by_organization(&self, _: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        unreachable!()
    }
    async fn find_by_status(&self, _: &SyncJobStatus) -> Result<Vec<SyncJob>, DomainError> {
        unreachable!()
    }
    async fn find_running(&self) -> Result<Vec<SyncJob>, DomainError> {
        unreachable!()
    }
    async fn find_recent(&self, _: i64) -> Result<Vec<SyncJob>, DomainError> {
        unreachable!()
    }
    async fn count_by_external_link(&self, _: &Uuid) -> Result<i64, DomainError> {
        unreachable!()
    }
    async fn count_running(&self) -> Result<i64, DomainError> {
        unreachable!()
    }
}
#[async_trait]
impl SyncJobWriteRepository for Probe {
    async fn save(&self, job: &SyncJob) -> Result<SyncJob, DomainError> {
        self.job_writes.fetch_add(1, Ordering::SeqCst);
        *self.saved_job.lock().unwrap() = Some(job.clone());
        Ok(job.clone())
    }
    async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
        self.job_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
}
impl SyncJobRepository for Probe {}

#[async_trait]
impl ExternalLinkReadRepository for Probe {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalLink>, DomainError> {
        Ok((*id == self.link.id).then(|| self.link.clone()))
    }
    async fn find_by_organization(&self, _: &Uuid) -> Result<Vec<ExternalLink>, DomainError> {
        unreachable!()
    }
    async fn find_by_organization_and_provider(
        &self,
        _: &Uuid,
        _: &Uuid,
    ) -> Result<Option<ExternalLink>, DomainError> {
        unreachable!()
    }
    async fn find_sync_enabled(&self) -> Result<Vec<ExternalLink>, DomainError> {
        unreachable!()
    }
    async fn find_needing_sync(&self, _: i64) -> Result<Vec<ExternalLink>, DomainError> {
        unreachable!()
    }
    async fn count_by_organization(&self, _: &Uuid) -> Result<i64, DomainError> {
        unreachable!()
    }
}
#[async_trait]
impl ExternalLinkWriteRepository for Probe {
    async fn save(&self, _: &ExternalLink) -> Result<ExternalLink, DomainError> {
        self.link_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
    async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
        self.link_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
}
impl ExternalLinkRepository for Probe {}

#[async_trait]
impl OrganizationReadRepository for Probe {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Organization>, DomainError> {
        let organization = self.organization.lock().unwrap();
        Ok((*id == organization.id).then(|| organization.clone()))
    }
    async fn find_by_slug(&self, _: &str) -> Result<Option<Organization>, DomainError> {
        unreachable!()
    }
    async fn find_by_owner(&self, _: &Uuid) -> Result<Vec<Organization>, DomainError> {
        unreachable!()
    }
    async fn find_by_user_membership(
        &self,
        _: &Uuid,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        unreachable!()
    }
    async fn search_by_name(
        &self,
        _: Option<PrincipalRef>,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        unreachable!()
    }
    async fn count(&self) -> Result<i64, DomainError> {
        unreachable!()
    }
}
#[async_trait]
impl OrganizationWriteRepository for Probe {
    async fn exists_by_slug(&self, _: &str) -> Result<bool, DomainError> {
        unreachable!()
    }
    async fn save(&self, _: &Organization) -> Result<Organization, DomainError> {
        self.organization_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
    async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
        self.organization_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
}
impl OrganizationRepository for Probe {}

#[async_trait]
impl OrganizationService for Probe {
    async fn create_organization(
        &self,
        _: &Organization,
        _: &str,
    ) -> Result<Organization, DomainError> {
        unreachable!()
    }
    async fn update_organization(
        &self,
        _: Uuid,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: Option<Value>,
    ) -> Result<Organization, DomainError> {
        self.organization_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
    async fn delete_organization(&self, _: Uuid) -> Result<(), DomainError> {
        self.organization_writes.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }
    async fn get_organization(&self, _: &Uuid) -> Result<Organization, DomainError> {
        unreachable!()
    }
    async fn get_organization_by_slug(&self, _: &str) -> Result<Organization, DomainError> {
        unreachable!()
    }
    async fn list_user_organizations(
        &self,
        _: &Uuid,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        unreachable!()
    }
    async fn search_organizations(
        &self,
        _: &str,
        _: Option<PrincipalRef>,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        unreachable!()
    }
}

#[async_trait]
impl InvitationService for Probe {
    async fn create_invitation_by_email(
        &self,
        organization_id: Uuid,
        email: String,
        roles: Vec<RolePermission>,
        invited_by: Uuid,
        message: Option<String>,
        _: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        self.invitation_creates.fetch_add(1, Ordering::SeqCst);
        assert_eq!(invited_by, self.organization.lock().unwrap().owner_user_id);
        OrganizationInvitation::new(organization_id, email, roles, invited_by, message)
    }
    async fn prepare_invitation_by_email(
        &self,
        _: Uuid,
        _: String,
        _: Vec<RolePermission>,
        _: Uuid,
        _: Option<String>,
        _: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        unreachable!()
    }
    async fn create_invitation_by_user(
        &self,
        _: Uuid,
        _: Uuid,
        _: Vec<RolePermission>,
        _: Uuid,
        _: Option<String>,
        _: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        unreachable!()
    }
    async fn accept_invitation(
        &self,
        _: String,
        _: Uuid,
        _: &str,
    ) -> Result<OrganizationMember, DomainError> {
        unreachable!()
    }
    async fn cancel_invitation(&self, _: Uuid) -> Result<(), DomainError> {
        unreachable!()
    }
    async fn get_invitation(&self, _: Uuid) -> Result<OrganizationInvitation, DomainError> {
        unreachable!()
    }
    async fn get_invitation_by_organization_invited_aggregate_id(
        &self,
        _: Uuid,
        _: &str,
    ) -> Result<OrganizationInvitation, DomainError> {
        self.invitation_reads.fetch_add(1, Ordering::SeqCst);
        Err(DomainError::entity_not_found(
            "OrganizationInvitation",
            "fixture",
        ))
    }
    async fn list_invitations(&self, _: Uuid) -> Result<Vec<OrganizationInvitation>, DomainError> {
        unreachable!()
    }
    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError> {
        unreachable!()
    }
}

#[async_trait]
impl ExternalProviderClient for Probe {
    async fn get_members(&self, _: &str, _: &Value) -> Result<Vec<ExternalMember>, DomainError> {
        use crate::entity::{
            permission::{Permission, PermissionLevel},
            resource::Resource,
        };
        self.provider_calls.fetch_add(1, Ordering::SeqCst);
        let organization_id = self.organization.lock().unwrap().id;
        Ok(vec![ExternalMember {
            external_id: "fixture-member".into(),
            username: "fixture-member".into(),
            email: Some("member@example.test".into()),
            display_name: None,
            avatar_url: None,
            roles: vec![RolePermission::new(
                None,
                None,
                organization_id,
                &Permission::new(PermissionLevel::Read, None, None),
                &Resource::new("organization".into(), None, None),
                None,
            )],
            is_active: true,
            provider_source: "fixture".into(),
        }])
    }
    async fn validate_config(&self, _: &str, _: &Value) -> Result<(), DomainError> {
        unreachable!()
    }
    async fn test_connection(&self, _: &str, _: &Value) -> Result<bool, DomainError> {
        unreachable!()
    }
    async fn sync_members(&self, _: &str, _: &Value) -> Result<Vec<ExternalMember>, DomainError> {
        unreachable!()
    }
    async fn get_organization_info(
        &self,
        _: &str,
        _: &Value,
    ) -> Result<ExternalOrganizationInfo, DomainError> {
        unreachable!()
    }
    async fn is_member(&self, _: &str, _: &Value, _: &str) -> Result<bool, DomainError> {
        unreachable!()
    }
}
