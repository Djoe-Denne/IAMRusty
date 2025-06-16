use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add password field to users table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::PasswordHash).string().null())
                    .to_owned(),
            )
            .await?;

        // Create user_email_verification table for email verification tokens
        manager
            .create_table(
                Table::create()
                    .table(UserEmailVerification::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserEmailVerification::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserEmailVerification::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(UserEmailVerification::VerificationToken)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(UserEmailVerification::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserEmailVerification::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on verification token for fast lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_email_verification_token")
                    .table(UserEmailVerification::Table)
                    .col(UserEmailVerification::VerificationToken)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup operations
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_email_verification_expires_at")
                    .table(UserEmailVerification::Table)
                    .col(UserEmailVerification::ExpiresAt)
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
                    .name("idx_email_verification_expires_at")
                    .table(UserEmailVerification::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_email_verification_token")
                    .table(UserEmailVerification::Table)
                    .to_owned(),
            )
            .await?;

        // Drop user_email_verification table
        manager
            .drop_table(Table::drop().table(UserEmailVerification::Table).to_owned())
            .await?;

        // Remove password_hash column from users table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::PasswordHash)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Users {
    Table,
    PasswordHash,
}

#[derive(Iden)]
enum UserEmailVerification {
    Table,
    Id,
    Email,
    VerificationToken,
    ExpiresAt,
    CreatedAt,
}
