//! Organizations SeaORM Entity

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organizations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    #[sea_orm(unique)]
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub owner_user_id: Uuid,
    pub settings: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::organization_members::Entity")]
    OrganizationMembers,
    #[sea_orm(has_many = "super::organization_roles::Entity")]
    OrganizationRoles,
    #[sea_orm(has_many = "super::organization_invitations::Entity")]
    OrganizationInvitations,
    #[sea_orm(has_many = "super::external_links::Entity")]
    ExternalLinks,
}

impl Related<super::organization_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganizationMembers.def()
    }
}

impl Related<super::organization_roles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganizationRoles.def()
    }
}

impl Related<super::organization_invitations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganizationInvitations.def()
    }
}

impl Related<super::external_links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalLinks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {} 