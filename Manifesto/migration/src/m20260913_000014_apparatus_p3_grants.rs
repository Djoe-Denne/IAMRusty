//! Colonne additive P3 `grant_revision` sur `apparatus_bindings` + table
//! `apparatus_capability_consents`.
//!
//! `apparatus_bindings.grant_revision` : BIGINT NOT NULL DEFAULT 0, CHECK >= 0.
//! Distinct de `desired_generation`.
//!
//! `apparatus_capability_consents` : consentement par (component_id, capability),
//! status ∈ consented|revoked, FK CASCADE vers `project_components`.
//!
//! Réversible : `down` retire la table et la colonne, sans drop de
//! `apparatus_bindings`. Aucun second UUID public. Pas de table KV.

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
                        ColumnDef::new(ApparatusBindings::GrantRevision)
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
ALTER TABLE apparatus_bindings
    ADD CONSTRAINT chk_apparatus_bindings_grant_revision_non_negative
        CHECK (grant_revision >= 0);
            ",
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApparatusCapabilityConsents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApparatusCapabilityConsents::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCapabilityConsents::ComponentId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCapabilityConsents::Capability)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCapabilityConsents::Status)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCapabilityConsents::GrantRevision)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_apparatus_capability_consents_component")
                            .from(
                                ApparatusCapabilityConsents::Table,
                                ApparatusCapabilityConsents::ComponentId,
                            )
                            .to(ProjectComponents::Table, ProjectComponents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            r"
ALTER TABLE apparatus_capability_consents
    ADD CONSTRAINT chk_apparatus_capability_consents_status
        CHECK (status IN ('consented', 'revoked')),
    ADD CONSTRAINT chk_apparatus_capability_consents_grant_revision_non_negative
        CHECK (grant_revision >= 0),
    ADD CONSTRAINT uq_apparatus_capability_consents_component_capability
        UNIQUE (component_id, capability);
            ",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ApparatusCapabilityConsents::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ApparatusBindings::Table)
                    .drop_column(ApparatusBindings::GrantRevision)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ApparatusBindings {
    Table,
    GrantRevision,
}

#[derive(DeriveIden)]
enum ApparatusCapabilityConsents {
    Table,
    Id,
    ComponentId,
    Capability,
    Status,
    GrantRevision,
}

#[derive(DeriveIden)]
enum ProjectComponents {
    Table,
    Id,
}
