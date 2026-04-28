use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Projects::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Projects::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Projects::Description).text().null())
                    .col(
                        ColumnDef::new(Projects::Status)
                            .string_len(50)
                            .not_null()
                            .default("draft")
                            .check(Expr::col(Projects::Status).is_in([
                                "draft",
                                "active",
                                "archived",
                                "suspended",
                            ])),
                    )
                    .col(
                        ColumnDef::new(Projects::OwnerType)
                            .string_len(50)
                            .not_null()
                            .check(
                                Expr::col(Projects::OwnerType).is_in(["personal", "organization"]),
                            ),
                    )
                    .col(ColumnDef::new(Projects::OwnerId).uuid().not_null())
                    .col(ColumnDef::new(Projects::CreatedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(Projects::Visibility)
                            .string_len(50)
                            .not_null()
                            .default("private")
                            .check(
                                Expr::col(Projects::Visibility)
                                    .is_in(["private", "internal", "public"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Projects::ExternalCollaborationEnabled)
                            .boolean()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Projects::DataClassification)
                            .string_len(50)
                            .default("internal"),
                    )
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::PublishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_projects_owner")
                    .table(Projects::Table)
                    .col(Projects::OwnerType)
                    .col(Projects::OwnerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_projects_status")
                    .table(Projects::Table)
                    .col(Projects::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_projects_created_by")
                    .table(Projects::Table)
                    .col(Projects::CreatedBy)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Projects {
    Table,
    Id,
    Name,
    Description,
    Status,
    OwnerType,
    OwnerId,
    CreatedBy,
    Visibility,
    ExternalCollaborationEnabled,
    DataClassification,
    CreatedAt,
    UpdatedAt,
    PublishedAt,
}
