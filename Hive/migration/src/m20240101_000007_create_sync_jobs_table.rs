use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SyncJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SyncJobs::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::OrganizationExternalLinkId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SyncJobs::JobType).string_len(50).not_null())
                    .col(ColumnDef::new(SyncJobs::Status).string_len(20).not_null())
                    .col(
                        ColumnDef::new(SyncJobs::ItemsProcessed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::ItemsCreated)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::ItemsUpdated)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::ItemsFailed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(SyncJobs::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(SyncJobs::Details)
                            .json_binary()
                            .not_null()
                            .default("'{}'".to_owned()),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sync_jobs_external_link_id")
                            .from(SyncJobs::Table, SyncJobs::OrganizationExternalLinkId)
                            .to(ExternalLinks::Table, ExternalLinks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sync_jobs_external_link_id")
                    .table(SyncJobs::Table)
                    .col(SyncJobs::OrganizationExternalLinkId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sync_jobs_status_started_at")
                    .table(SyncJobs::Table)
                    .col(SyncJobs::Status)
                    .col(SyncJobs::StartedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sync_jobs_created_at")
                    .table(SyncJobs::Table)
                    .col(SyncJobs::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SyncJobs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ExternalLinks {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum SyncJobs {
    Table,
    Id,
    OrganizationExternalLinkId,
    JobType,
    Status,
    ItemsProcessed,
    ItemsCreated,
    ItemsUpdated,
    ItemsFailed,
    StartedAt,
    CompletedAt,
    ErrorMessage,
    Details,
    CreatedAt,
}
