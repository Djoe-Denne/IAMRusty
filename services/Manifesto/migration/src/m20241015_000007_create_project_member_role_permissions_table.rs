use sea_orm_migration::prelude::*;

use super::m20241015_000003_create_project_members_table::ProjectMembers;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProjectMemberRolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::MemberId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::RolePermissionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_member_role_permissions_member_id")
                            .from(
                                ProjectMemberRolePermissions::Table,
                                ProjectMemberRolePermissions::MemberId,
                            )
                            .to(ProjectMembers::Table, ProjectMembers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_member_role_permissions_role_permission_id")
                            .from(
                                ProjectMemberRolePermissions::Table,
                                ProjectMemberRolePermissions::RolePermissionId,
                            )
                            .to(RolePermissions::Table, RolePermissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on member_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_member_role_permissions_member_id")
                    .table(ProjectMemberRolePermissions::Table)
                    .col(ProjectMemberRolePermissions::MemberId)
                    .to_owned(),
            )
            .await?;

        // Create index on role_permission_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_member_role_permissions_role_permission_id")
                    .table(ProjectMemberRolePermissions::Table)
                    .col(ProjectMemberRolePermissions::RolePermissionId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate role permission assignments
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_member_role_permissions_unique")
                    .table(ProjectMemberRolePermissions::Table)
                    .col(ProjectMemberRolePermissions::MemberId)
                    .col(ProjectMemberRolePermissions::RolePermissionId)
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
                    .table(ProjectMemberRolePermissions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ProjectMemberRolePermissions {
    Table,
    Id,
    MemberId,
    RolePermissionId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RolePermissions {
    Table,
    Id,
}
