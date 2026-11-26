use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // PostgreSQL BIGINT cannot represent the full unsigned 64-bit range.
        // No ordering queries are needed; store the canonical decimal form.
        for table in ["users", "refresh_tokens"] {
            manager
                .get_connection()
                .execute_unprepared(&format!(
                    "ALTER TABLE {table} ADD COLUMN auth_epoch TEXT NOT NULL DEFAULT '0' \
                     CHECK (auth_epoch COLLATE \"C\" ~ '^(0|[1-9][0-9]{{0,19}})$' \
                         AND (length(auth_epoch) < 20 \
                              OR auth_epoch COLLATE \"C\" <= '18446744073709551615'))"
                ))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Never erase a revocation fence by downgrading the schema after recovery.
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    LOCK TABLE users, refresh_tokens IN ACCESS EXCLUSIVE MODE;
                    IF EXISTS (SELECT 1 FROM users WHERE auth_epoch <> '0')
                        OR EXISTS (SELECT 1 FROM refresh_tokens WHERE auth_epoch <> '0')
                    THEN
                        RAISE EXCEPTION 'cannot drop an advanced authentication epoch';
                    END IF;
                    ALTER TABLE refresh_tokens DROP COLUMN auth_epoch;
                    ALTER TABLE users DROP COLUMN auth_epoch;
                END; $$",
            )
            .await?;
        Ok(())
    }
}
