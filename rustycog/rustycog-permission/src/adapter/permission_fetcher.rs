use async_trait::async_trait;
use uuid::Uuid;
use rustycog_core::error::DomainError;
use crate::Permission;

#[async_trait]
pub trait PermissionsFetch: Send + Sync {
    async fn fetch_permissions(
        &self, 
        user_id: Uuid, 
        resource_ids: Vec<Uuid>
    ) -> Result<Vec<Permission>, DomainError>;
}