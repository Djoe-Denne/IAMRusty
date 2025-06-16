use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create user_emails table
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

        // Add unique constraint on email (across all users)
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

        // Add constraint to ensure only one primary email per user
        manager
            .create_index(
                Index::create()
                    .name("idx_user_emails_user_primary_unique")
                    .table(UserEmails::Table)
                    .col(UserEmails::UserId)
                    .col(UserEmails::IsPrimary)
                    .unique()
                    // Note: This partial unique index would ideally only apply when is_primary = true
                    // but SeaORM migration doesn't support partial indexes yet
                    // We'll handle this constraint in application logic
                    .to_owned(),
            )
            .await?;

        // Remove the unique constraint from users.email (drop the index first)
        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_email_unique")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;

        // Remove email column from users table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Email)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add email column back to users table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::Email).string().not_null().default(""))
                    .to_owned(),
            )
            .await?;

        // Add back unique constraint on users.email
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

        // Drop user_emails table
        manager
            .drop_table(Table::drop().table(UserEmails::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
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
