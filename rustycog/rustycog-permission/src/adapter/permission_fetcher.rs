use async_trait::async_trait;
use uuid::Uuid;
use rustycog_core::error::DomainError;
use crate::{Permission, ResourceId};

#[async_trait]
pub trait PermissionsFetcher: Send + Sync {
    async fn fetch_permissions(
        &self, 
        user_id: Option<Uuid>, 
        resource_ids: Vec<ResourceId>
    ) -> Result<Vec<Permission>, DomainError>;
}