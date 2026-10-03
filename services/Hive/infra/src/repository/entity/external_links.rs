//! External Links `SeaORM` Entity

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "external_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub provider_id: Uuid,
    pub provider_config: Value,
    pub sync_enabled: bool,
    pub sync_settings: Value,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,
    pub sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Organization,
    #[sea_orm(
        belongs_to = "super::external_providers::Entity",
        from = "Column::ProviderId",
        to = "super::external_providers::Column::Id"
    )]
    ExternalProvider,
    #[sea_orm(has_many = "super::sync_jobs::Entity")]
    SyncJobs,
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::external_providers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalProvider.def()
    }
}

impl Related<super::sync_jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SyncJobs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
