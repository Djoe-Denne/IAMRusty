use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // The PostgreSQL Migrator runs this inside a transaction. Keep writers
        // fenced until both the backfill and trigger installation commit:
        // existing writes finish before the scan, later writes use the trigger.
        //
        // These are publisher relations only. Deployment must separately create
        // a trusted, initially empty subscriber and one ordered subscription for
        // both relations; this migration provisions no replication roles/grants.
        // The invoker-rights trigger requires the IAM writer role to have DML
        // and SELECT access to the projection (normally it owns the migrated
        // relations). Do not grant the gateway access to this IAM database.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DO $migration$
                BEGIN
                    IF current_setting('transaction_isolation') <> 'read committed' THEN
                        RAISE EXCEPTION 'session liveness backfill requires READ COMMITTED';
                    END IF;
                END;
                $migration$;

                -- A fresh statement snapshot after the lock includes every
                -- account change committed by writers that preceded the fence.
                LOCK TABLE public.users IN SHARE ROW EXCLUSIVE MODE;

                CREATE TABLE public.iam_session_liveness_epochs (
                    subject UUID PRIMARY KEY,
                    auth_epoch TEXT NOT NULL
                        CHECK (auth_epoch COLLATE "C" ~ '^(0|[1-9][0-9]{0,19})$'
                            AND (length(auth_epoch) < 20
                                OR auth_epoch COLLATE "C" <= '18446744073709551615'))
                );

                CREATE TABLE public.iam_session_liveness_checkpoints (
                    challenge UUID PRIMARY KEY
                );

                CREATE FUNCTION public.iam_sync_session_liveness_epoch()
                RETURNS trigger
                LANGUAGE plpgsql
                SECURITY INVOKER
                SET search_path = pg_catalog, pg_temp
                AS $$
                BEGIN
                    IF TG_OP = 'TRUNCATE' THEN
                        DELETE FROM public.iam_session_liveness_epochs;
                        RETURN NULL;
                    END IF;
                    IF TG_OP = 'DELETE' THEN
                        DELETE FROM public.iam_session_liveness_epochs
                        WHERE subject = OLD.id;
                        RETURN NULL;
                    END IF;
                    IF TG_OP = 'UPDATE' THEN
                        IF OLD.id IS DISTINCT FROM NEW.id THEN
                            DELETE FROM public.iam_session_liveness_epochs
                            WHERE subject = OLD.id;
                        END IF;
                    END IF;
                    INSERT INTO public.iam_session_liveness_epochs (subject, auth_epoch)
                    VALUES (NEW.id, NEW.auth_epoch)
                    ON CONFLICT (subject) DO UPDATE
                    SET auth_epoch = EXCLUDED.auth_epoch;
                    RETURN NULL;
                END;
                $$;

                CREATE TRIGGER iam_session_liveness_account_created_or_deleted
                AFTER INSERT OR DELETE ON public.users
                FOR EACH ROW
                EXECUTE FUNCTION public.iam_sync_session_liveness_epoch();

                CREATE TRIGGER iam_session_liveness_account_epoch_changed
                AFTER UPDATE ON public.users
                FOR EACH ROW
                WHEN (OLD.id IS DISTINCT FROM NEW.id
                    OR OLD.auth_epoch COLLATE "C" IS DISTINCT FROM NEW.auth_epoch COLLATE "C")
                EXECUTE FUNCTION public.iam_sync_session_liveness_epoch();

                CREATE TRIGGER iam_session_liveness_accounts_truncated
                AFTER TRUNCATE ON public.users
                FOR EACH STATEMENT
                EXECUTE FUNCTION public.iam_sync_session_liveness_epoch();

                INSERT INTO public.iam_session_liveness_epochs (subject, auth_epoch)
                SELECT id, auth_epoch FROM public.users;
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Disable replica admission and retire its subscription before downgrade.
        // Drop only derived state: never erase users.auth_epoch or refresh fences.
        // As in up(), the PostgreSQL migration transaction holds the writer lock.
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- DROP TRIGGER requires ACCESS EXCLUSIVE. Acquire it directly
                -- rather than upgrading while an existing IAM writer finishes.
                LOCK TABLE public.users IN ACCESS EXCLUSIVE MODE;
                DROP TRIGGER iam_session_liveness_accounts_truncated ON public.users;
                DROP TRIGGER iam_session_liveness_account_epoch_changed ON public.users;
                DROP TRIGGER iam_session_liveness_account_created_or_deleted ON public.users;
                DROP FUNCTION public.iam_sync_session_liveness_epoch();
                DROP TABLE public.iam_session_liveness_checkpoints;
                DROP TABLE public.iam_session_liveness_epochs;
                "#,
            )
            .await?;
        Ok(())
    }
}
