//! `SeaORM` entity for `signing_keys`

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "signing_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub kid: String,
    pub algorithm: String,
    pub trust_scope: String,
    pub issuer: String,
    pub provider_type: String,
    pub provider_key_ref: String,
    pub credential_ref: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub public_key: String,
    pub status: String,
    pub organization_id: Option<Uuid>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
