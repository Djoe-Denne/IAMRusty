//! Colonne additive `declared_capabilities` sur `apparatus_bindings`.
//!
//! JSONB NOT NULL DEFAULT `[]`. Aucune lecture de manifeste d'artifact :
//! la liste est fournie à l'INSERT (tests) ou reste vide
//! (fail-closed si `declared_capabilities` est vide / à la frontière de capacité).
//!
//! Réversible : `down` retire uniquement cette colonne.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ApparatusBindings::Table)
                    .add_column(
                        ColumnDef::new(ApparatusBindings::DeclaredCapabilities)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'::jsonb")),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ApparatusBindings::Table)
                    .drop_column(ApparatusBindings::DeclaredCapabilities)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ApparatusBindings {
    Table,
    DeclaredCapabilities,
}
