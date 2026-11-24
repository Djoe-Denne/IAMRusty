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
                r#"
                CREATE TABLE profile_email_verifications (
                    address_id UUID PRIMARY KEY REFERENCES user_emails(id) ON DELETE CASCADE,
                    user_id UUID NOT NULL,
                    email TEXT COLLATE "C" NOT NULL,
                    token_digest BYTEA NOT NULL UNIQUE CHECK (octet_length(token_digest) = 32),
                    issued_at TIMESTAMPTZ NOT NULL,
                    expires_at TIMESTAMPTZ NOT NULL,
                    confirmed_at TIMESTAMPTZ NULL,
                    CHECK (expires_at > issued_at)
                );

                CREATE FUNCTION iam_invalidate_profile_email_proof()
                RETURNS trigger LANGUAGE plpgsql AS $$
                BEGIN
                    DELETE FROM profile_email_verifications WHERE address_id = OLD.id;
                    RETURN NULL;
                END;
                $$;

                CREATE TRIGGER iam_profile_email_address_replaced
                AFTER UPDATE OF email, user_id ON user_emails
                FOR EACH ROW
                WHEN (OLD.user_id IS DISTINCT FROM NEW.user_id
                      OR OLD.email COLLATE "C" IS DISTINCT FROM NEW.email COLLATE "C")
                EXECUTE FUNCTION iam_invalidate_profile_email_proof();
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TRIGGER iam_profile_email_address_replaced ON user_emails;
                 DROP FUNCTION iam_invalidate_profile_email_proof();
                 DROP TABLE profile_email_verifications;",
            )
            .await?;
        Ok(())
    }
}
