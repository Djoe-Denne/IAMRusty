use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExampleEntity::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExampleEntity::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ExampleEntity::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExampleEntity::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ExampleEntity::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(ExampleEntity::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ExampleEntity::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Add indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_example_entity_name")
                    .table(ExampleEntity::Table)
                    .col(ExampleEntity::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_example_entity_status")
                    .table(ExampleEntity::Table)
                    .col(ExampleEntity::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_example_entity_created_at")
                    .table(ExampleEntity::Table)
                    .col(ExampleEntity::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExampleEntity::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ExampleEntity {
    Table,
    Id,
    Name,
    Description,
    Status,
    CreatedAt,
    UpdatedAt,
} 