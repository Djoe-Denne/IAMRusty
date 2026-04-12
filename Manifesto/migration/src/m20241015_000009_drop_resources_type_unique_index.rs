use sea_orm_migration::prelude::*;

/// Migration to drop the unique index on resources.resource_type
/// This index was originally created for generic resource types (project, component, member)
/// but it prevents creating multiple component instance resources which share the same resource_type.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the unique index on resource_type
        manager
            .drop_index(
                Index::drop()
                    .name("idx_resources_type")
                    .table(Resources::Table)
                    .to_owned(),
            )
            .await?;

        // Create a non-unique index for query performance
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_resources_type_non_unique")
                    .table(Resources::Table)
                    .col(Resources::ResourceType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the non-unique index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_resources_type_non_unique")
                    .table(Resources::Table)
                    .to_owned(),
            )
            .await?;

        // Recreate the unique index
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

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Resources {
    Table,
    ResourceType,
}

