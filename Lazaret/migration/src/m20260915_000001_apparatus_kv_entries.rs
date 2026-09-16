//! Table `apparatus_kv_entries` (Lazaret DB, no FK to Manifesto).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApparatusKvEntries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApparatusKvEntries::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ApparatusKvEntries::BindingId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ApparatusKvEntries::Key).text().not_null())
                    .col(
                        ColumnDef::new(ApparatusKvEntries::Value)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusKvEntries::CasVersion)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        db.execute_unprepared(
            r"
ALTER TABLE apparatus_kv_entries
    ADD CONSTRAINT chk_apparatus_kv_entries_cas_version_non_negative
        CHECK (cas_version >= 0),
    ADD CONSTRAINT uq_apparatus_kv_entries_binding_key
        UNIQUE (binding_id, key);
            ",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ApparatusKvEntries::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ApparatusKvEntries {
    Table,
    Id,
    BindingId,
    Key,
    Value,
    CasVersion,
}
