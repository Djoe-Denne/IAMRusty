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
                        ColumnDef::new(OrganizationMemberRolePermissions::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::RolePermissionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_role_permissions_organization_id")
                            .from(
                                OrganizationMemberRolePermissions::Table,
                                OrganizationMemberRolePermissions::OrganizationId,
                            )
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_role_permissions_role_permission_id")
                            .from(
                                OrganizationMemberRolePermissions::Table,
                                OrganizationMemberRolePermissions::RolePermissionId,
                            )
                            .to(RolePermissions::Table, RolePermissions::Id)
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
                    .name("idx_org_member_role_permissions_org_user")
                    .table(OrganizationMemberRolePermissions::Table)
                    .col(OrganizationMemberRolePermissions::OrganizationId)
                    .col(OrganizationMemberRolePermissions::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on role_permission_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_role_permissions_role_permission_id")
                    .table(OrganizationMemberRolePermissions::Table)
                    .col(OrganizationMemberRolePermissions::RolePermissionId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate role permission assignments
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_role_permissions_unique")
                    .table(OrganizationMemberRolePermissions::Table)
                    .col(OrganizationMemberRolePermissions::OrganizationId)
                    .col(OrganizationMemberRolePermissions::UserId)
                    .col(OrganizationMemberRolePermissions::RolePermissionId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(OrganizationMemberRolePermissions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum OrganizationMemberRolePermissions {
    Table,
    Id,
    OrganizationId,
    UserId,
    RolePermissionId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum RolePermissions {
    Table,
    Id,
}
