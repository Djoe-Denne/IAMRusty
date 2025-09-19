use async_trait::async_trait;
use uuid::Uuid;

use crate::service::NotificationService;
use rustycog_core::error::DomainError;
use rustycog_permission::{Permission, PermissionsFetcher, ResourceId};
use std::sync::Arc;
use tracing::debug;

pub struct ResourcePermissionFetcher<NS> 
where
    NS: NotificationService,
{
    notification_service: Arc<NS>,
    _resource_names: Vec<String>,
}

impl<NS> ResourcePermissionFetcher<NS>
where
    NS: NotificationService,
{
    pub fn new(notification_service: Arc<NS>, resource_names: Vec<String>) -> Self {
        Self { notification_service, _resource_names: resource_names }
    }
}

#[async_trait]
impl<NS> PermissionsFetcher for ResourcePermissionFetcher<NS>
where
    NS: NotificationService + Send + Sync,
{
    async fn fetch_permissions(
        &self,
        user_id: Uuid,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        debug!("Checking permissions for user {:?} on {:?} resources", user_id, resource_ids.len());

        // If the user owns any of the provided notification IDs, grant Write permission.
        for resource_id in resource_ids {
            let notification_id = resource_id.id();
            if self
                .notification_service
                .user_has_notification(user_id, notification_id)
                .await
            {
                return Ok(vec![Permission::Write]);
            }
        }

        Ok(vec![])
    }
}
