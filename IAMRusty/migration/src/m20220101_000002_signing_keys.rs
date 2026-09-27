use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_signing_keys_table(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SigningKeys::Table).to_owned())
            .await
    }
}

async fn create_signing_keys_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SigningKeys::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(SigningKeys::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(SigningKeys::Kid)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(SigningKeys::Algorithm).string().not_null())
                .col(ColumnDef::new(SigningKeys::TrustScope).string().not_null())
                .col(ColumnDef::new(SigningKeys::Issuer).string().not_null())
                .col(
                    ColumnDef::new(SigningKeys::ProviderType)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SigningKeys::ProviderKeyRef)
                        .string()
                        .not_null(),
                )
                .col(ColumnDef::new(SigningKeys::CredentialRef).string().null())
                .col(ColumnDef::new(SigningKeys::PublicKey).text().not_null())
                .col(ColumnDef::new(SigningKeys::Status).string().not_null())
                .col(ColumnDef::new(SigningKeys::OrganizationId).uuid().null())
                .col(
                    ColumnDef::new(SigningKeys::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(SigningKeys::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await
}

#[derive(Iden)]
enum SigningKeys {
    Table,
    Id,
    Kid,
    Algorithm,
    TrustScope,
    Issuer,
    ProviderType,
    ProviderKeyRef,
    CredentialRef,
    PublicKey,
    Status,
    OrganizationId,
    CreatedAt,
    UpdatedAt,
}
