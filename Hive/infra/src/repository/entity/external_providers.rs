//! External Providers SeaORM Entity

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "external_providers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub provider_type: String,
    pub name: String,
    pub config_schema: Option<Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::external_links::Entity")]
    ExternalLinks,
}

impl Related<super::external_links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalLinks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
