//! Exercise both use-case persistence paths with the real domain service.
//! All collaborators are private in-memory probes; no runtime fixture is started.

use super::*;
use hive_domain::{
    port::{
        service::{ExternalMember, ExternalOrganizationInfo, ExternalProviderClient},
        ExternalLinkReadRepository, ExternalLinkRepository, ExternalLinkWriteRepository,
        OrganizationReadRepository, OrganizationRepository, OrganizationWriteRepository,
        SyncJobReadRepository, SyncJobRepository, SyncJobWriteRepository,
    },
    service::{InvitationService, OrganizationService, SyncServiceImpl},
    ExternalLink, Organization, OrganizationInvitation, OrganizationMember, RolePermission,
    SyncJob, SyncJobStatus,
};
use rustycog::permission::{PrincipalPolicy, PrincipalRef};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

struct Probe {
    links: [ExternalLink; 2],
    foreign_link_present: AtomicBool,
    link_reads: AtomicUsize,
    running_reads: AtomicUsize,
    direct_saves: AtomicUsize,
    outbox_saves: AtomicUsize,
    publishes: AtomicUsize,
    records: AtomicUsize,
    jobs: Mutex<Vec<SyncJob>>,
    events: Mutex<Vec<SyncJobStartedEvent>>,
}

impl Probe {
    fn capture_event(&self, event: &dyn DomainEvent) {
        let event = serde_json::from_str(&event.to_json().unwrap()).unwrap();
        let HiveDomainEvent::SyncJobStarted(event) = event else {
            panic!("expected a sync job started event");
        };
        self.events.lock().unwrap().push(event);
    }

    fn assert_no_effects(&self) {
        assert_eq!(self.running_reads.load(Ordering::SeqCst), 0);
        assert_eq!(self.direct_saves.load(Ordering::SeqCst), 0);
        assert_eq!(self.outbox_saves.load(Ordering::SeqCst), 0);
        assert_eq!(self.publishes.load(Ordering::SeqCst), 0);
        assert_eq!(self.records.load(Ordering::SeqCst), 0);
        assert!(self.jobs.lock().unwrap().is_empty());
        assert!(self.events.lock().unwrap().is_empty());
    }
}

fn fixture(transactional: bool) -> (SyncJobUseCaseImpl, Arc<Probe>) {
    let links = [("A", "organization-a"), ("B", "organization-b")].map(|(name, slug)| {
        let organization =
            Organization::new(name.into(), slug.into(), None, Uuid::new_v4()).unwrap();
        let mut link = ExternalLink::new(
            organization.id,
            None,
            Uuid::new_v4(),
            Some("fixture".into()),
            serde_json::json!({"organization": name}),
            None,
        )
        .unwrap();
        link.enable_sync();
        link
    });
    let probe = Arc::new(Probe {
        links,
        foreign_link_present: AtomicBool::new(true),
        link_reads: AtomicUsize::new(0),
        running_reads: AtomicUsize::new(0),
        direct_saves: AtomicUsize::new(0),
        outbox_saves: AtomicUsize::new(0),
        publishes: AtomicUsize::new(0),
        records: AtomicUsize::new(0),
        jobs: Mutex::new(Vec::new()),
        events: Mutex::new(Vec::new()),
    });
    let service = Arc::new(SyncServiceImpl::new(
        probe.clone(),
        probe.clone(),
        probe.clone(),
        probe.clone(),
        probe.clone(),
        probe.clone(),
        Arc::new(PrincipalPolicy::new("https://platform.example/iam", false).unwrap()),
    ));
    let usecase = if transactional {
        SyncJobUseCaseImpl::new_with_outbox_unit_of_work(service, probe.clone(), probe.clone())
    } else {
        SyncJobUseCaseImpl::new(service, probe.clone())
    };
    (usecase, probe)
}

fn request(external_link_id: Uuid) -> StartSyncJobRequest {
    StartSyncJobRequest {
        external_link_id,
        job_type: "members_only".into(),
        options: None,
    }
}

#[tokio::test]
async fn sync_job_rejects_external_link_from_another_organization_before_persist() {
    for transactional in [false, true] {
        let (usecase, probe) = fixture(transactional);
        let organization_a = probe.links[0].organization_id;
        let link_b = probe.links[1].id;
        let actor = Uuid::new_v4();
        assert_ne!(organization_a, probe.links[1].organization_id);
        assert!(probe.links[1].is_sync_enabled());

        let foreign_error = usecase
            .start_sync_job(organization_a, request(link_b), actor)
            .await
            .expect_err("an authorized path must not admit another organization's link");
        assert!(matches!(
            &foreign_error,
            ApplicationError::Domain(DomainError::EntityNotFound { entity_type, id })
                if entity_type == "ExternalLink" && id == &link_b.to_string()
        ));
        assert_eq!(probe.link_reads.load(Ordering::SeqCst), 1);
        probe.assert_no_effects();

        // The same request must not disclose whether the foreign link exists.
        probe.foreign_link_present.store(false, Ordering::SeqCst);
        let missing_error = usecase
            .start_sync_job(organization_a, request(link_b), actor)
            .await
            .expect_err("an absent link must also be refused");
        assert_eq!(foreign_error.to_string(), missing_error.to_string());
        assert_eq!(probe.link_reads.load(Ordering::SeqCst), 2);
        probe.assert_no_effects();

        let link_a = &probe.links[0];
        let response = usecase
            .start_sync_job(organization_a, request(link_a.id), actor)
            .await
            .expect("refusing a foreign link must not prevent a legitimate job");
        assert_eq!(response.external_link_id, link_a.id);
        assert_eq!(response.organization_id, organization_a);
    }
}

#[tokio::test]
async fn sync_job_accepts_external_link_from_authorized_organization() {
    for transactional in [false, true] {
        let (usecase, probe) = fixture(transactional);
        let link_a = &probe.links[0];
        let response = usecase
            .start_sync_job(link_a.organization_id, request(link_a.id), Uuid::new_v4())
            .await
            .expect("the authorized organization's own enabled link remains usable");

        assert_eq!(response.organization_id, link_a.organization_id);
        assert_eq!(response.external_link_id, link_a.id);
        assert_eq!(response.job_type, "members_only");
        assert_eq!(response.status, "running");
        assert_eq!(probe.link_reads.load(Ordering::SeqCst), 1);
        assert_eq!(probe.running_reads.load(Ordering::SeqCst), 1);
        assert_eq!(
            probe.direct_saves.load(Ordering::SeqCst),
            usize::from(!transactional)
        );
        assert_eq!(
            probe.outbox_saves.load(Ordering::SeqCst),
            usize::from(transactional)
        );
        assert_eq!(
            probe.publishes.load(Ordering::SeqCst),
            usize::from(!transactional)
        );
        assert_eq!(probe.records.load(Ordering::SeqCst), 0);
        let jobs = probe.jobs.lock().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, response.id);
        assert_eq!(jobs[0].organization_external_link_id, link_a.id);
        assert_eq!(jobs[0].status, SyncJobStatus::Running);
        let events = probe.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].organization_id, link_a.organization_id);
        assert_eq!(events[0].base.aggregate_id, link_a.organization_id);
        assert_eq!(events[0].external_link_id, link_a.id);
        assert_eq!(events[0].sync_job_id, response.id);
        assert_eq!(events[0].job_type, "members_only");
    }
}

#[async_trait]
impl SyncJobReadRepository for Probe {
    async fn find_by_id(&self, _: &Uuid) -> Result<Option<SyncJob>, DomainError> {
        panic!("unexpected job lookup");
    }

    async fn find_running_by_external_link(&self, _: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        self.running_reads.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    async fn find_by_external_link(&self, _: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        panic!("unexpected job lookup");
    }

    async fn find_by_organization(&self, _: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        panic!("unexpected job lookup");
    }

    async fn find_by_status(&self, _: &SyncJobStatus) -> Result<Vec<SyncJob>, DomainError> {
        panic!("unexpected job lookup");
    }

    async fn find_running(&self) -> Result<Vec<SyncJob>, DomainError> {
        panic!("unexpected job lookup");
    }

    async fn find_recent(&self, _: i64) -> Result<Vec<SyncJob>, DomainError> {
        panic!("unexpected job lookup");
    }

    async fn count_by_external_link(&self, _: &Uuid) -> Result<i64, DomainError> {
        panic!("unexpected job count");
    }

    async fn count_running(&self) -> Result<i64, DomainError> {
        panic!("unexpected job count");
    }
}

#[async_trait]
impl SyncJobWriteRepository for Probe {
    async fn save(&self, job: &SyncJob) -> Result<SyncJob, DomainError> {
        self.direct_saves.fetch_add(1, Ordering::SeqCst);
        self.jobs.lock().unwrap().push(job.clone());
        Ok(job.clone())
    }

    async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
        panic!("unexpected job deletion");
    }
}

impl SyncJobRepository for Probe {}

#[async_trait]
impl ExternalLinkReadRepository for Probe {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalLink>, DomainError> {
        self.link_reads.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .links
            .iter()
            .find(|link| {
                link.id == *id
                    && (link.id != self.links[1].id
                        || self.foreign_link_present.load(Ordering::SeqCst))
            })
            .cloned())
    }

    async fn find_by_organization(&self, _: &Uuid) -> Result<Vec<ExternalLink>, DomainError> {
        panic!("unexpected link lookup");
    }

    async fn find_by_organization_and_provider(
        &self,
        _: &Uuid,
        _: &Uuid,
    ) -> Result<Option<ExternalLink>, DomainError> {
        panic!("unexpected link lookup");
    }

    async fn find_sync_enabled(&self) -> Result<Vec<ExternalLink>, DomainError> {
        panic!("unexpected link lookup");
    }

    async fn find_needing_sync(&self, _: i64) -> Result<Vec<ExternalLink>, DomainError> {
        panic!("unexpected link lookup");
    }

    async fn count_by_organization(&self, _: &Uuid) -> Result<i64, DomainError> {
        panic!("unexpected link count");
    }
}

#[async_trait]
impl ExternalLinkWriteRepository for Probe {
    async fn save(&self, _: &ExternalLink) -> Result<ExternalLink, DomainError> {
        panic!("unexpected link mutation");
    }

    async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
        panic!("unexpected link deletion");
    }
}

impl ExternalLinkRepository for Probe {}

#[async_trait]
impl OrganizationReadRepository for Probe {
    async fn find_by_id(&self, _: &Uuid) -> Result<Option<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn find_by_slug(&self, _: &str) -> Result<Option<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn find_by_owner(&self, _: &Uuid) -> Result<Vec<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn find_by_user_membership(
        &self,
        _: &Uuid,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn search_by_name(
        &self,
        _: Option<PrincipalRef>,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn count(&self) -> Result<i64, DomainError> {
        panic!("unexpected organization count");
    }
}

#[async_trait]
impl OrganizationWriteRepository for Probe {
    async fn exists_by_slug(&self, _: &str) -> Result<bool, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn save(&self, _: &Organization) -> Result<Organization, DomainError> {
        panic!("unexpected organization mutation");
    }

    async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
        panic!("unexpected organization deletion");
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
        panic!("unexpected organization creation");
    }

    async fn update_organization(
        &self,
        _: Uuid,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: Option<Value>,
    ) -> Result<Organization, DomainError> {
        panic!("unexpected organization mutation");
    }

    async fn delete_organization(&self, _: Uuid) -> Result<(), DomainError> {
        panic!("unexpected organization deletion");
    }

    async fn get_organization(&self, _: &Uuid) -> Result<Organization, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn get_organization_by_slug(&self, _: &str) -> Result<Organization, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn list_user_organizations(
        &self,
        _: &Uuid,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }

    async fn search_organizations(
        &self,
        _: &str,
        _: Option<PrincipalRef>,
        _: u32,
        _: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        panic!("unexpected organization lookup");
    }
}

#[async_trait]
impl InvitationService for Probe {
    async fn create_invitation_by_email(
        &self,
        _: Uuid,
        _: String,
        _: Vec<RolePermission>,
        _: Uuid,
        _: Option<String>,
        _: Option<i64>,
    ) -> Result<OrganizationInvitation, DomainError> {
        panic!("unexpected invitation creation");
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
        panic!("unexpected invitation preparation");
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
        panic!("unexpected invitation creation");
    }

    async fn accept_invitation(
        &self,
        _: String,
        _: Uuid,
        _: &str,
    ) -> Result<OrganizationMember, DomainError> {
        panic!("unexpected invitation acceptance");
    }

    async fn cancel_invitation(&self, _: Uuid) -> Result<(), DomainError> {
        panic!("unexpected invitation cancellation");
    }

    async fn get_invitation(&self, _: Uuid) -> Result<OrganizationInvitation, DomainError> {
        panic!("unexpected invitation lookup");
    }

    async fn get_invitation_by_organization_invited_aggregate_id(
        &self,
        _: Uuid,
        _: &str,
    ) -> Result<OrganizationInvitation, DomainError> {
        panic!("unexpected invitation lookup");
    }

    async fn list_invitations(&self, _: Uuid) -> Result<Vec<OrganizationInvitation>, DomainError> {
        panic!("unexpected invitation lookup");
    }

    async fn cleanup_expired_invitations(&self) -> Result<u32, DomainError> {
        panic!("unexpected invitation cleanup");
    }
}

#[async_trait]
impl ExternalProviderClient for Probe {
    async fn get_members(&self, _: &str, _: &Value) -> Result<Vec<ExternalMember>, DomainError> {
        panic!("unexpected provider request");
    }

    async fn validate_config(&self, _: &str, _: &Value) -> Result<(), DomainError> {
        panic!("unexpected provider request");
    }

    async fn test_connection(&self, _: &str, _: &Value) -> Result<bool, DomainError> {
        panic!("unexpected provider request");
    }

    async fn sync_members(&self, _: &str, _: &Value) -> Result<Vec<ExternalMember>, DomainError> {
        panic!("unexpected provider request");
    }

    async fn get_organization_info(
        &self,
        _: &str,
        _: &Value,
    ) -> Result<ExternalOrganizationInfo, DomainError> {
        panic!("unexpected provider request");
    }

    async fn is_member(&self, _: &str, _: &Value, _: &str) -> Result<bool, DomainError> {
        panic!("unexpected provider request");
    }
}

#[async_trait]
impl EventPublisher<DomainError> for Probe {
    async fn publish(&self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        self.publishes.fetch_add(1, Ordering::SeqCst);
        self.capture_event(event);
        Ok(())
    }

    async fn publish_batch(&self, _: &[Box<dyn DomainEvent>]) -> Result<(), DomainError> {
        panic!("unexpected batch publication");
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        panic!("unexpected publisher health check");
    }
}

#[async_trait]
impl HiveOutboxUnitOfWork for Probe {
    async fn create_organization(
        &self,
        _: Organization,
        _: String,
        _: Box<dyn DomainEvent + 'static>,
    ) -> Result<Organization, ApplicationError> {
        panic!("unexpected organization creation");
    }

    async fn update_organization(
        &self,
        _: Organization,
        _: Box<dyn DomainEvent + 'static>,
    ) -> Result<Organization, ApplicationError> {
        panic!("unexpected organization mutation");
    }

    async fn delete_organization(
        &self,
        _: Uuid,
        _: hive_events::OrganizationDeletedEvent,
    ) -> Result<(), ApplicationError> {
        panic!("unexpected organization deletion");
    }

    async fn add_member(
        &self,
        _: Uuid,
        _: Uuid,
        _: String,
        _: Vec<RolePermission>,
        _: Option<Uuid>,
        _: Box<dyn DomainEvent + 'static>,
    ) -> Result<OrganizationMember, ApplicationError> {
        panic!("unexpected member creation");
    }

    async fn remove_member(
        &self,
        _: Uuid,
        _: Uuid,
        _: String,
        _: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        panic!("unexpected member removal");
    }

    async fn save_invitation(
        &self,
        _: OrganizationInvitation,
        _: Box<dyn DomainEvent + 'static>,
    ) -> Result<OrganizationInvitation, ApplicationError> {
        panic!("unexpected invitation mutation");
    }

    async fn save_external_link(
        &self,
        _: ExternalLink,
        _: Box<dyn DomainEvent + 'static>,
    ) -> Result<ExternalLink, ApplicationError> {
        panic!("unexpected link mutation");
    }

    async fn save_sync_job(
        &self,
        job: SyncJob,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<SyncJob, ApplicationError> {
        self.outbox_saves.fetch_add(1, Ordering::SeqCst);
        self.jobs.lock().unwrap().push(job.clone());
        self.capture_event(event.as_ref());
        Ok(job)
    }

    async fn record_event(
        &self,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        self.records.fetch_add(1, Ordering::SeqCst);
        self.capture_event(event.as_ref());
        Ok(())
    }
}
