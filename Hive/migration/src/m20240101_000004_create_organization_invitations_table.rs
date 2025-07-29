use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganizationInvitations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganizationInvitations::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Email)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::RoleId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::InvitedByUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Token)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::AcceptedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Message)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organization_invitations_organization_id")
                            .from(OrganizationInvitations::Table, OrganizationInvitations::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organization_invitations_role_id")
                            .from(OrganizationInvitations::Table, OrganizationInvitations::RoleId)
                            .to(OrganizationRoles::Table, OrganizationRoles::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_token")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::Token)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_email_status")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::Email)
                    .col(OrganizationInvitations::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_organization_id")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::OrganizationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_expires_at")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on organization_id + email + status for pending invitations
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_org_email_status")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::OrganizationId)
                    .col(OrganizationInvitations::Email)
                    .col(OrganizationInvitations::Status)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrganizationInvitations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum OrganizationRoles {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum OrganizationInvitations {
    Table,
    Id,
    OrganizationId,
    Email,
    RoleId,
    InvitedByUserId,
    Token,
    Status,
    ExpiresAt,
    AcceptedAt,
    Message,
    CreatedAt,
} 