use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "project_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub source: String,
    pub added_by: Option<Uuid>,
    pub added_at: DateTimeWithTimeZone,
    pub removed_at: Option<DateTimeWithTimeZone>,
    pub removal_reason: Option<String>,
    pub grace_period_ends_at: Option<DateTimeWithTimeZone>,
    pub last_access_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::projects::Entity",
        from = "Column::ProjectId",
        to = "super::projects::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Projects,
    #[sea_orm(has_many = "super::project_member_role_permissions::Entity")]
    ProjectMemberRolePermissions,
}

impl Related<super::projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<super::project_member_role_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectMemberRolePermissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
