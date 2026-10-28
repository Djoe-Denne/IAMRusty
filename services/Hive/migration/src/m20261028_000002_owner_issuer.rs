//! Additive provenance only; no automatic attribution/backfill of existing owners.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE organizations ADD COLUMN owner_issuer VARCHAR(256) NULL",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Dropping provenance cannot safely restore a vulnerable multi-issuer reader.
        Err(DbErr::Custom(
            "Coordinated authority rollback requires the approved maintenance procedure".into(),
        ))
    }
}
