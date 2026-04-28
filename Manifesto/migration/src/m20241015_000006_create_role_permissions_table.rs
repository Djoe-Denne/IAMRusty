use sea_orm_migration::prelude::*;

use super::m20241015_000001_create_projects_table::Projects;

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
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(RolePermissions::Name).string_len(100).null())
                    .col(ColumnDef::new(RolePermissions::ProjectId).uuid().not_null())
                    .col(
                        ColumnDef::new(RolePermissions::PermissionId)
                            .string_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::ResourceId)
                            .string_len(36)
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
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_project_id")
                            .from(RolePermissions::Table, RolePermissions::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
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

        // Create index on project_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_project_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::ProjectId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate permission-resource combinations per project
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_unique_combo")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::ProjectId)
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
    ProjectId,
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
