use sea_orm_migration::prelude::{
    async_trait, ConnectionTrait, DbErr, DeriveMigrationName, MigrationTrait, SchemaManager,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE totp_credentials (
                user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                enrollment_id UUID NOT NULL,
                sealed_seed BYTEA NOT NULL CHECK (octet_length(sealed_seed) = 48),
                expires_at TIMESTAMPTZ NOT NULL,
                last_used_step BIGINT CHECK (last_used_step >= 0),
                login_window_started_at TIMESTAMPTZ,
                login_attempts INTEGER NOT NULL DEFAULT 0 CHECK (login_attempts BETWEEN 0 AND 5)
            )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Only for explicit teardown of a disposable database. Not a supported
        // rollback after enrollment: an old binary cannot enforce the factor.
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE totp_credentials")
            .await?;
        Ok(())
    }
}
