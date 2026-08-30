pub mod external_link;
pub mod invitation;
pub mod member;
pub mod organization;
pub mod role;
pub mod sync_job;

use crate::ApplicationError;
use hive_domain::{
    ExternalLink, Organization, OrganizationInvitation, OrganizationMember, RolePermission, SyncJob,
};
use rustycog::events::DomainEvent;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait HiveOutboxUnitOfWork: Send + Sync {
    /// Persist a new organization (roles + owner) and its outbox event in one write transaction.
    async fn create_organization(
        &self,
        organization: Organization,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<Organization, ApplicationError>;

    async fn update_organization(
        &self,
        organization: Organization,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<Organization, ApplicationError>;

    async fn delete_organization(
        &self,
        organization_id: Uuid,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError>;

    async fn add_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        roles: Vec<RolePermission>,
        added_by_user_id: Option<Uuid>,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<OrganizationMember, ApplicationError>;

    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError>;

    async fn save_invitation(
        &self,
        invitation: OrganizationInvitation,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<OrganizationInvitation, ApplicationError>;

    async fn save_external_link(
        &self,
        link: ExternalLink,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<ExternalLink, ApplicationError>;

    async fn save_sync_job(
        &self,
        job: SyncJob,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<SyncJob, ApplicationError>;

    /// Isolated outbox insert (tests / dispatcher). Prefer the persist methods.
    async fn record_event(
        &self,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError>;
}

// Re-export all use case traits for convenience
pub use external_link::{ExternalLinkUseCase, ExternalLinkUseCaseImpl};
pub use invitation::{InvitationUseCase, InvitationUseCaseImpl};
pub use member::{MemberUseCase, MemberUseCaseImpl};
pub use organization::{OrganizationUseCase, OrganizationUseCaseImpl};
pub use role::{RoleUseCase, RoleUseCaseImpl};
pub use sync_job::{SyncJobUseCase, SyncJobUseCaseImpl};
