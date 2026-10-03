use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Each organization seeds the same permission/resource combinations.
        // Keep uniqueness within an organization, not across organizations.
        manager
            .create_index(
                Index::create()
                    .name("idx_role_permissions_org_unique_combo")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::OrganizationId)
                    .col(RolePermissions::PermissionId)
                    .col(RolePermissions::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_role_permissions_unique_combo")
                    .table(RolePermissions::Table)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Restore the old constraint first. If multiple organizations now use
        // the same combination, rollback fails without deleting their roles.
        manager
            .create_index(
                Index::create()
                    .name("idx_role_permissions_unique_combo")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::PermissionId)
                    .col(RolePermissions::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_role_permissions_org_unique_combo")
                    .table(RolePermissions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum RolePermissions {
    Table,
    OrganizationId,
    PermissionId,
    ResourceId,
}
