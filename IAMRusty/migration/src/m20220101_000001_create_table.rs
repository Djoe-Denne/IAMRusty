use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create users table (final schema with all features)
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Users::Username).string().null()) // nullable for incomplete registration
                    .col(ColumnDef::new(Users::AvatarUrl).string())
                    .col(ColumnDef::new(Users::PasswordHash).string().null()) // for email/password auth
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create user_emails table (multiple emails per user)
        manager
            .create_table(
                Table::create()
                    .table(UserEmails::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserEmails::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserEmails::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserEmails::Email).string().not_null())
                    .col(
                        ColumnDef::new(UserEmails::IsPrimary)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserEmails::IsVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserEmails::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UserEmails::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_emails_user_id")
                            .from(UserEmails::Table, UserEmails::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint on email (across all users)
        manager
            .create_index(
                Index::create()
                    .name("idx_user_emails_email_unique")
                    .table(UserEmails::Table)
                    .col(UserEmails::Email)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create provider_tokens table (with provider_user_id)
        manager
            .create_table(
                Table::create()
                    .table(ProviderTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProviderTokens::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ProviderTokens::UserId).uuid().not_null())
                    .col(ColumnDef::new(ProviderTokens::Provider).string().not_null())
                    .col(ColumnDef::new(ProviderTokens::ProviderUserId).string().not_null()) // moved from users table
                    .col(
                        ColumnDef::new(ProviderTokens::AccessToken)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ProviderTokens::RefreshToken).string())
                    .col(ColumnDef::new(ProviderTokens::ExpiresIn).integer())
                    .col(
                        ColumnDef::new(ProviderTokens::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ProviderTokens::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-provider_tokens-user_id")
                            .from(ProviderTokens::Table, ProviderTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add unique constraint on provider + provider_user_id
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

        // Create refresh_tokens table
        manager
            .create_table(
                Table::create()
                    .table(RefreshTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RefreshTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RefreshTokens::UserId).uuid().not_null())
                    .col(ColumnDef::new(RefreshTokens::Token).text().not_null())
                    .col(
                        ColumnDef::new(RefreshTokens::IsValid)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(RefreshTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(RefreshTokens::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-refresh_tokens-user_id")
                            .from(RefreshTokens::Table, RefreshTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
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

        // Create indexes for email verification
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

        // Create indexes for password reset tokens
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
        // Drop all tables in reverse dependency order
        manager
            .drop_table(Table::drop().table(PasswordResetTokens::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(UserEmailVerification::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RefreshTokens::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ProviderTokens::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(UserEmails::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    AvatarUrl,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserEmails {
    Table,
    Id,
    UserId,
    Email,
    IsPrimary,
    IsVerified,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ProviderTokens {
    Table,
    Id,
    UserId,
    Provider,
    ProviderUserId,
    AccessToken,
    RefreshToken,
    ExpiresIn,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RefreshTokens {
    Table,
    Id,
    UserId,
    Token,
    IsValid,
    CreatedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum UserEmailVerification {
    Table,
    Id,
    Email,
    VerificationToken,
    ExpiresAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum PasswordResetTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    CreatedAt,
    UsedAt,
}
