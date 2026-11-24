use sea_orm::entity::prelude::*;

#[derive(Clone, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "totp_credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub enrollment_id: Uuid,
    pub sealed_seed: Vec<u8>,
    pub expires_at: DateTimeUtc,
    pub last_used_step: Option<i64>,
    pub login_window_started_at: Option<DateTimeUtc>,
    pub login_attempts: i32,
}

// SeaORM requires Debug on models. Do not expose the credential payload.
impl std::fmt::Debug for Model {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TotpCredential")
            .finish_non_exhaustive()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
