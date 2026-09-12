//! Extension Apparatus 1:1 sur project_components.id (digest nullable, source legacy|managed).
//!
//! Table `apparatus_bindings` : FK `component_id` → `project_components.id` (`CASCADE`),
//! contrainte `UNIQUE` 1:1 sur `component_id`, `digest` nullable non résolu,
//! `source` ∈ `legacy|managed`. PK surrogate entier interne (`id`) : aucun second
//! UUID public, l'identité d'installation reste `project_components.id`.
//! Additive et réversible (`down` retire la table) ; aucune réécriture de données legacy.
//!
//! Note : `component_id` ne peut pas cumuler `PRIMARY KEY` + `UNIQUE` — Postgres
//! déduplique la contrainte redondante et l'entrée `UNIQUE` disparaît de
//! `information_schema` (sondé le 2026-09-12). D'où le surrogate `id`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApparatusBindings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApparatusBindings::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ApparatusBindings::ComponentId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ApparatusBindings::Digest)
                            .string_len(128)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusBindings::Source)
                            .string_len(20)
                            .not_null()
                            .check(
                                Expr::col(ApparatusBindings::Source).is_in(["legacy", "managed"]),
                            ),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_apparatus_bindings_component")
                            .from(ApparatusBindings::Table, ApparatusBindings::ComponentId)
                            .to(ProjectComponents::Table, ProjectComponents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ApparatusBindings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ApparatusBindings {
    Table,
    Id,
    ComponentId,
    Digest,
    Source,
}

#[derive(DeriveIden)]
enum ProjectComponents {
    Table,
    Id,
}
