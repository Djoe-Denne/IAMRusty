use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add provider_user_id to provider_tokens table
        manager
            .alter_table(
                Table::alter()
                    .table(ProviderTokens::Table)
                    .add_column(
                        ColumnDef::new(ProviderTokens::ProviderUserId)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        // Remove provider_user_id from users table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::ProviderUserId)
                    .to_owned(),
            )
            .await?;

        // Make email required in users table (it should be the linking field)
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(
                        ColumnDef::new(Users::Email)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add unique constraint on email
        manager
            .create_index(
                Index::create()
                    .name("idx_users_email_unique")
                    .table(Users::Table)
                    .col(Users::Email)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Add unique constraint on provider + provider_user_id in provider_tokens
        manager
            .create_index(
                Index::create()
                    .name("idx_provider_tokens_provider_user_unique")
                    .table(ProviderTokens::Table)
                    .col(ProviderTokens::Provider)
                    .col(ProviderTokens::ProviderUserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indices
        manager
            .drop_index(
                Index::drop()
                    .name("idx_provider_tokens_provider_user_unique")
                    .table(ProviderTokens::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_email_unique")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;

        // Make email optional again
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(
                        ColumnDef::new(Users::Email)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add back provider_user_id to users table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::ProviderUserId)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        // Remove provider_user_id from provider_tokens table
        manager
            .alter_table(
                Table::alter()
                    .table(ProviderTokens::Table)
                    .drop_column(ProviderTokens::ProviderUserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Email,
    ProviderUserId,
}

#[derive(DeriveIden)]
enum ProviderTokens {
    Table,
    Provider,
    ProviderUserId,
} 