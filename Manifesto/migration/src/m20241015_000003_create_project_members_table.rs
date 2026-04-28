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
                    .table(ProjectMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(ProjectMembers::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(ProjectMembers::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(ProjectMembers::Source)
                            .string_len(50)
                            .not_null()
                            .default("direct")
                            .check(Expr::col(ProjectMembers::Source).is_in([
                                "direct",
                                "org_cascade",
                                "invitation",
                                "third_party_sync",
                            ])),
                    )
                    .col(ColumnDef::new(ProjectMembers::AddedBy).uuid().null())
                    .col(
                        ColumnDef::new(ProjectMembers::AddedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::RemovedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::RemovalReason)
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::GracePeriodEndsAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::LastAccessAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_members_project")
                            .from(ProjectMembers::Table, ProjectMembers::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("project_members_unique")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::ProjectId)
                    .col(ProjectMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_project")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_user")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::UserId)
                    .to_owned(),
            )
            .await?;

        // Create partial index for active members
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_active")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::ProjectId)
                    .col(ProjectMembers::RemovedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProjectMembers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ProjectMembers {
    Table,
    Id,
    ProjectId,
    UserId,
    Source,
    AddedBy,
    AddedAt,
    RemovedAt,
    RemovalReason,
    GracePeriodEndsAt,
    LastAccessAt,
}
