//! Organization Roles SeaORM Entity

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organization_roles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system_default: bool,
    pub created_at: DateTime<Utc>,
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
    #[sea_orm(has_many = "super::organization_members::Entity")]
    OrganizationMembers,
    #[sea_orm(has_many = "super::organization_invitations::Entity")]
    OrganizationInvitations,
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::organization_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganizationMembers.def()
    }
}

impl Related<super::organization_invitations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganizationInvitations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {} 