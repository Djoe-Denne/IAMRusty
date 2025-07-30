use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Permissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Permissions::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Permissions::Level).string_len(20).not_null())
                    .col(ColumnDef::new(Permissions::Description).text().null())
                    .col(
                        ColumnDef::new(Permissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on level
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_permissions_level")
                    .table(Permissions::Table)
                    .col(Permissions::Level)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Insert default permission levels
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Permissions::Table)
                    .columns([Permissions::Level, Permissions::Description])
                    .values_panic(["read".into(), "Read-only access to resources".into()])
                    .values_panic(["write".into(), "Read and write access to resources".into()])
                    .values_panic([
                        "admin".into(),
                        "Full administrative access to resources".into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Permissions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Permissions {
    Table,
    Id,
    Level,
    Description,
    CreatedAt,
}
