use sea_orm_migration::prelude::{
    async_trait, ConnectionTrait, DbErr, DeriveMigrationName, MigrationTrait, SchemaManager,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Permit reapplication after a metadata-only down that preserves the column.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE oauth_transactions ADD COLUMN IF NOT EXISTS authentication_nonce TEXT NULL",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Intentionally irreversible DDL: operational rollback keeps the nullable
        // column and restores only the old binary/config, never stored secrets.
        // Migrator metadata may be rolled back; up remains safe to reapply.
        Ok(())
    }
}
