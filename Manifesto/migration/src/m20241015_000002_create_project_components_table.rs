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
                    .table(ProjectComponents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectComponents::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ProjectId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ComponentType)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::Status)
                            .string_len(50)
                            .not_null()
                            .default("pending")
                            .check(Expr::col(ProjectComponents::Status).is_in([
                                "pending",
                                "configured",
                                "active",
                                "disabled",
                            ])),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::AddedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ConfiguredAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ActivatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::DisabledAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_components_project")
                            .from(ProjectComponents::Table, ProjectComponents::ProjectId)
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
                    .name("project_components_unique")
                    .table(ProjectComponents::Table)
                    .col(ProjectComponents::ProjectId)
                    .col(ProjectComponents::ComponentType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_components_project")
                    .table(ProjectComponents::Table)
                    .col(ProjectComponents::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_components_status")
                    .table(ProjectComponents::Table)
                    .col(ProjectComponents::ProjectId)
                    .col(ProjectComponents::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProjectComponents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProjectComponents {
    Table,
    Id,
    ProjectId,
    ComponentType,
    Status,
    AddedAt,
    ConfiguredAt,
    ActivatedAt,
    DisabledAt,
}
