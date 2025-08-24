use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExternalLinks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExternalLinks::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ExternalLinks::ProviderId).uuid().not_null())
                    .col(
                        ColumnDef::new(ExternalLinks::ProviderConfig)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::SyncEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::SyncSettings)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb")),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::LastSyncAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::LastSyncStatus)
                            .string_len(20)
                            .null(),
                    )
                    .col(ColumnDef::new(ExternalLinks::SyncError).text().null())
                    .col(
                        ColumnDef::new(ExternalLinks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_external_links_organization_id")
                            .from(ExternalLinks::Table, ExternalLinks::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_external_links_provider_id")
                            .from(ExternalLinks::Table, ExternalLinks::ProviderId)
                            .to(ExternalProviders::Table, ExternalProviders::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_links_organization_id")
                    .table(ExternalLinks::Table)
                    .col(ExternalLinks::OrganizationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_links_sync_enabled_last_sync")
                    .table(ExternalLinks::Table)
                    .col(ExternalLinks::SyncEnabled)
                    .col(ExternalLinks::LastSyncAt)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on organization_id + provider_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_links_org_provider")
                    .table(ExternalLinks::Table)
                    .col(ExternalLinks::OrganizationId)
                    .col(ExternalLinks::ProviderId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExternalLinks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ExternalProviders {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ExternalLinks {
    Table,
    Id,
    OrganizationId,
    ProviderId,
    ProviderConfig,
    SyncEnabled,
    SyncSettings,
    LastSyncAt,
    LastSyncStatus,
    SyncError,
    CreatedAt,
    UpdatedAt,
}
