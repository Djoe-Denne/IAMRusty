pub mod external_link;
pub mod invitation;
pub mod member;
pub mod organization;
pub mod sync_job;

use crate::ApplicationError;
use rustycog_events::DomainEvent;

#[async_trait::async_trait]
pub trait HiveOutboxUnitOfWork: Send + Sync {
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
pub use sync_job::{SyncJobUseCase, SyncJobUseCaseImpl};
