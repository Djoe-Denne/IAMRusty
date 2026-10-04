use sea_orm::entity::prelude::*;

#[derive(Clone, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "oauth_transactions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub nonce_hash: Vec<u8>,
    pub state_hash: Vec<u8>,
    pub browser_nonce_hash: Vec<u8>,
    pub provider: String,
    pub operation: String,
    pub target_user_id: Option<Uuid>,
    pub redirect_uri: String,
    pub pkce_verifier: Option<String>,
    pub expires_at: DateTimeWithTimeZone,
    pub consumed_at: Option<DateTimeWithTimeZone>,
}

// SeaORM requires Debug on models; deliberately redact the complete row.
impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OAuthTransactionModel([redacted])")
    }
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
