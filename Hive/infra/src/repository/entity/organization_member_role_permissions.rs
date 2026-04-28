//! Organization Member Role Permissions SeaORM Entity

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organization_member_role_permissions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub member_id: Uuid,
    pub role_permission_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organization_members::Entity",
        from = "Column::MemberId",
        to = "super::organization_members::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    OrganizationMember,
    #[sea_orm(
        belongs_to = "super::role_permissions::Entity",
        from = "Column::RolePermissionId",
        to = "super::role_permissions::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    RolePermission,
}

impl Related<super::organization_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganizationMember.def()
    }
}

impl Related<super::role_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RolePermission.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
