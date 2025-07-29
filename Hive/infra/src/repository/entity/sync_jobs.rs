//! Sync Jobs SeaORM Entity

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_external_link_id: Uuid,
    pub job_type: String,
    pub status: String,
    pub items_processed: i32,
    pub items_created: i32,
    pub items_updated: i32,
    pub items_failed: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub details: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::external_links::Entity",
        from = "Column::OrganizationExternalLinkId",
        to = "super::external_links::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    ExternalLink,
}

impl Related<super::external_links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalLink.def()
    }
}

impl ActiveModelBehavior for ActiveModel {} 