use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Resources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Resources::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Resources::ResourceType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Resources::Name).string_len(100).not_null())
                    .col(ColumnDef::new(Resources::Description).text().null())
                    .col(
                        ColumnDef::new(Resources::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on resource_type
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_resources_type")
                    .table(Resources::Table)
                    .col(Resources::ResourceType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Insert default resource types with IDs matching names used by permission fetcher
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Resources::Table)
                    .columns([
                        Resources::Id,
                        Resources::ResourceType,
                        Resources::Name,
                        Resources::Description,
                    ])
                    .values_panic([
                        "organization".into(),
                        "organization".into(),
                        "organization".into(),
                        "Organization management resources".into(),
                    ])
                    .values_panic([
                        "member".into(),
                        "member".into(),
                        "member".into(),
                        "Organization member management".into(),
                    ])
                    .values_panic([
                        "external_link".into(),
                        "external_link".into(),
                        "external_link".into(),
                        "External link management".into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Resources::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Resources {
    Table,
    Id,
    ResourceType,
    Name,
    Description,
    CreatedAt,
}
