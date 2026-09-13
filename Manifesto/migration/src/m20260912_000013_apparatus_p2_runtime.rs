//! Colonnes additives P2 sur `apparatus_bindings` + table interne `apparatus_cleanup_jobs`.
//!
//! `apparatus_bindings` : desired_generation, observed_generation, observed_digest,
//! lease_epoch, lease_owner, lease_expires_at, next_retry_at, retry_count, last_error_code.
//!
//! `apparatus_cleanup_jobs` : component_id, project_id, desired_generation, digest,
//! completed_at (pas de FK).
//!
//! Réversible : `down` retire les 9 colonnes et la table cleanup, sans drop
//! de `apparatus_bindings`. Aucun second UUID public.

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
                        ColumnDef::new(ApparatusBindings::DesiredGeneration)
                            .big_integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(ApparatusBindings::DesiredGeneration).gte(0)),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::ObservedGeneration)
                            .big_integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(ApparatusBindings::ObservedGeneration).gte(0)),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::ObservedDigest)
                            .string_len(128)
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::LeaseEpoch)
                            .big_integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(ApparatusBindings::LeaseEpoch).gte(0)),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::LeaseOwner)
                            .string_len(64)
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::LeaseExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::NextRetryAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::RetryCount)
                            .integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(ApparatusBindings::RetryCount).gte(0)),
                    )
                    .add_column(
                        ColumnDef::new(ApparatusBindings::LastErrorCode)
                            .string_len(64)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        db.execute_unprepared(
            r"
ALTER TABLE apparatus_bindings
    ADD CONSTRAINT chk_apparatus_bindings_observed_le_desired
        CHECK (observed_generation <= desired_generation),
    ADD CONSTRAINT chk_apparatus_bindings_lease_owned
        CHECK (
            (lease_owner = '' AND lease_expires_at IS NULL)
            OR (lease_owner <> '' AND lease_expires_at IS NOT NULL)
        );
            ",
        )
        .await?;

        db.execute_unprepared(
            r"
CREATE INDEX IF NOT EXISTS idx_apparatus_bindings_managed_due
    ON apparatus_bindings (next_retry_at)
    WHERE source = 'managed' AND desired_generation > observed_generation;
            ",
        )
        .await?;

        db.execute_unprepared(
            r"
CREATE INDEX IF NOT EXISTS idx_apparatus_bindings_lease_expiry
    ON apparatus_bindings (lease_expires_at)
    WHERE source = 'managed' AND lease_owner <> '';
            ",
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApparatusCleanupJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::ComponentId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::ProjectId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::DesiredGeneration)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::Digest)
                            .string_len(128)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::NextRetryAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::RetryCount)
                            .integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(ApparatusCleanupJobs::RetryCount).gte(0)),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::LastErrorCode)
                            .string_len(64)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApparatusCleanupJobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            r"
CREATE UNIQUE INDEX IF NOT EXISTS uq_apparatus_cleanup_jobs_open
    ON apparatus_cleanup_jobs (component_id)
    WHERE completed_at IS NULL;
            ",
        )
        .await?;

        db.execute_unprepared(
            r"
CREATE INDEX IF NOT EXISTS idx_apparatus_cleanup_jobs_due
    ON apparatus_cleanup_jobs (next_retry_at)
    WHERE completed_at IS NULL;
            ",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS apparatus_cleanup_jobs;")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_apparatus_bindings_managed_due;")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_apparatus_bindings_lease_expiry;")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ApparatusBindings::Table)
                    .drop_column(ApparatusBindings::DesiredGeneration)
                    .drop_column(ApparatusBindings::ObservedGeneration)
                    .drop_column(ApparatusBindings::ObservedDigest)
                    .drop_column(ApparatusBindings::LeaseEpoch)
                    .drop_column(ApparatusBindings::LeaseOwner)
                    .drop_column(ApparatusBindings::LeaseExpiresAt)
                    .drop_column(ApparatusBindings::NextRetryAt)
                    .drop_column(ApparatusBindings::RetryCount)
                    .drop_column(ApparatusBindings::LastErrorCode)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ApparatusBindings {
    Table,
    DesiredGeneration,
    ObservedGeneration,
    ObservedDigest,
    LeaseEpoch,
    LeaseOwner,
    LeaseExpiresAt,
    NextRetryAt,
    RetryCount,
    LastErrorCode,
}

#[derive(DeriveIden)]
enum ApparatusCleanupJobs {
    Table,
    Id,
    ComponentId,
    ProjectId,
    DesiredGeneration,
    Digest,
    NextRetryAt,
    RetryCount,
    LastErrorCode,
    CompletedAt,
    CreatedAt,
}
