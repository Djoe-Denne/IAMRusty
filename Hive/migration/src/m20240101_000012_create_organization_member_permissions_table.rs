use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganizationMemberRolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberPermissions::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberPermissions::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberPermissions::PermissionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberPermissions::ResourceId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberPermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_permissions_organization_id")
                            .from(OrganizationMemberPermissions::Table, OrganizationMemberPermissions::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_permissions_permission_id")
                            .from(OrganizationMemberPermissions::Table, OrganizationMemberPermissions::PermissionId)
                            .to(Permissions::Table, Permissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_permissions_resource_id")
                            .from(OrganizationMemberPermissions::Table, OrganizationMemberPermissions::ResourceId)
                            .to(Resources::Table, Resources::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on organization_id and user_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_permissions_org_user")
                    .table(OrganizationMemberPermissions::Table)
                    .col(OrganizationMemberPermissions::OrganizationId)
                    .col(OrganizationMemberPermissions::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on permission_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_permissions_permission_id")
                    .table(OrganizationMemberPermissions::Table)
                    .col(OrganizationMemberPermissions::PermissionId)
                    .to_owned(),
            )
            .await?;

        // Create index on resource_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_permissions_resource_id")
                    .table(OrganizationMemberPermissions::Table)
                    .col(OrganizationMemberPermissions::ResourceId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate permission assignments
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_permissions_unique")
                    .table(OrganizationMemberPermissions::Table)
                    .col(OrganizationMemberPermissions::OrganizationId)
                    .col(OrganizationMemberPermissions::UserId)
                    .col(OrganizationMemberPermissions::PermissionId)
                    .col(OrganizationMemberPermissions::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrganizationMemberPermissions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum OrganizationMemberPermissions {
    Table,
    Id,
    OrganizationId,
    UserId,
    PermissionId,
    ResourceId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Permissions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Resources {
    Table,
    Id,
} 