use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub created_by: Uuid,
    pub visibility: String,
    pub external_collaboration_enabled: bool,
    pub data_classification: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub published_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::project_components::Entity")]
    ProjectComponents,
    #[sea_orm(has_many = "super::project_members::Entity")]
    ProjectMembers,
}

impl Related<super::project_components::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectComponents.def()
    }
}

impl Related<super::project_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectMembers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
