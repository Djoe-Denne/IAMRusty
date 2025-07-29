use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RolePermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::PermissionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::ResourceId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )

                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_permission_id")
                            .from(RolePermissions::Table, RolePermissions::PermissionId)
                            .to(Permissions::Table, Permissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_resource_id")
                            .from(RolePermissions::Table, RolePermissions::ResourceId)
                            .to(Resources::Table, Resources::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on organization_role_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_organization_role_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::OrganizationRoleId)
                    .to_owned(),
            )
            .await?;

        // Create index on permission_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_permission_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::PermissionId)
                    .to_owned(),
            )
            .await?;

        // Create index on resource_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_resource_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::ResourceId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate permission-resource combinations
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_unique_combo")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::PermissionId)
                    .col(RolePermissions::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RolePermissions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum RolePermissions {
    Table,
    Id,
    Name,
    Description,
    PermissionId,
    ResourceId,
    CreatedAt,
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