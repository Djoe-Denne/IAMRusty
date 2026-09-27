use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_identities_user_issuer_kind")
                    .table(Identities::Table)
                    .col(Identities::UserId)
                    .col(Identities::Issuer)
                    .col(Identities::Kind)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_identities_user_issuer_kind")
                    .table(Identities::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Identities {
    Table,
    UserId,
    Issuer,
    Kind,
}
