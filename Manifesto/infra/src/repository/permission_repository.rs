use async_trait::async_trait;
use manifesto_domain::entity::Permission;
use manifesto_domain::port::PermissionReadRepository;
use manifesto_domain::value_objects::PermissionLevel;
use rustycog_core::error::DomainError;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::sync::Arc;
use tracing::debug;

use super::entity::{permissions, prelude::Permissions};

pub struct PermissionMapper;

impl PermissionMapper {
    pub fn to_domain(model: permissions::Model) -> Result<Permission, DomainError> {
        let level = PermissionLevel::from_str(&model.level)?;
        Ok(Permission {
            level,
            created_at: Some(model.created_at.naive_utc().and_utc()),
        })
    }
}

#[derive(Clone)]
pub struct PermissionReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl PermissionReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PermissionReadRepository for PermissionReadRepositoryImpl {
    async fn find_by_level(&self, level: &str) -> Result<Option<Permission>, DomainError> {
        debug!("Finding permission by level: {}", level);

        let permission = Permissions::find_by_id(level.to_string())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match permission {
            Some(p) => Ok(Some(PermissionMapper::to_domain(p)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Permission>, DomainError> {
        debug!("Finding all permissions");

        let permissions = Permissions::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        permissions
            .into_iter()
            .map(PermissionMapper::to_domain)
            .collect()
    }
}

// Note: No Write repository for Permissions - they are seeded and read-only

