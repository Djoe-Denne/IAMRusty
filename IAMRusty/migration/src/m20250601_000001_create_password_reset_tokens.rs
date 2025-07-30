use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create password_reset_tokens table
        manager
            .create_table(
                Table::create()
                    .table(PasswordResetTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PasswordResetTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PasswordResetTokens::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PasswordResetTokens::TokenHash)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(PasswordResetTokens::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PasswordResetTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(PasswordResetTokens::UsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_password_reset_tokens_user_id")
                            .from(PasswordResetTokens::Table, PasswordResetTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for fast lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_password_reset_tokens_user_id")
                    .table(PasswordResetTokens::Table)
                    .col(PasswordResetTokens::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on token_hash for fast lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_password_reset_tokens_token_hash")
                    .table(PasswordResetTokens::Table)
                    .col(PasswordResetTokens::TokenHash)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup operations
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_password_reset_tokens_expires_at")
                    .table(PasswordResetTokens::Table)
                    .col(PasswordResetTokens::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        // Create index on used_at for filtering used tokens
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_password_reset_tokens_used_at")
                    .table(PasswordResetTokens::Table)
                    .col(PasswordResetTokens::UsedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_password_reset_tokens_used_at")
                    .table(PasswordResetTokens::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_password_reset_tokens_expires_at")
                    .table(PasswordResetTokens::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_password_reset_tokens_token_hash")
                    .table(PasswordResetTokens::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_password_reset_tokens_user_id")
                    .table(PasswordResetTokens::Table)
                    .to_owned(),
            )
            .await?;

        // Drop password_reset_tokens table
        manager
            .drop_table(Table::drop().table(PasswordResetTokens::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum PasswordResetTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    CreatedAt,
    UsedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
