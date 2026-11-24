use sea_orm_migration::prelude::{
    async_trait, ConnectionTrait, DbErr, DeriveMigrationName, MigrationTrait, SchemaManager,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Existing duplicates stop the migration; it never rewrites account names.
        // PostgreSQL's default unique-index semantics permit multiple NULL usernames.
        manager
            .get_connection()
            .execute_unprepared("CREATE UNIQUE INDEX users_username_unique ON users (username)")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX users_username_unique")
            .await?;
        Ok(())
    }
}
