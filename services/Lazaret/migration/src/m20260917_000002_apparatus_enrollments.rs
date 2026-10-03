//! Table `apparatus_enrollments` (Lazaret DB, no FK to Manifesto).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApparatusEnrollments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApparatusEnrollments::Fingerprint)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ApparatusEnrollments::BindingId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusEnrollments::Instance)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusEnrollments::Release)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusEnrollments::Generation)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusEnrollments::GrantRevision)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        db.execute_unprepared(
            r"
ALTER TABLE apparatus_enrollments
    ADD CONSTRAINT uq_apparatus_enrollments_binding_id
        UNIQUE (binding_id);
            ",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ApparatusEnrollments::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ApparatusEnrollments {
    Table,
    Fingerprint,
    BindingId,
    Instance,
    Release,
    Generation,
    GrantRevision,
}
