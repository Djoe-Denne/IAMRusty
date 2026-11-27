use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            // Before this migration existing markers have no insertion order.
            // Enroll them before new markers; active proof is never carried
            // across an operator downgrade and re-bootstrap.
            .execute_unprepared(
                "LOCK TABLE public.iam_session_liveness_checkpoints IN SHARE ROW EXCLUSIVE MODE;
                CREATE TABLE public.iam_session_liveness_checkpoint_queue (
                    sequence BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                    challenge UUID NOT NULL UNIQUE REFERENCES
                        public.iam_session_liveness_checkpoints(challenge) ON DELETE CASCADE
                );
                INSERT INTO public.iam_session_liveness_checkpoint_queue(challenge)
                SELECT challenge FROM public.iam_session_liveness_checkpoints ORDER BY challenge;
                DELETE FROM public.iam_session_liveness_checkpoints WHERE challenge IN (
                    SELECT challenge FROM public.iam_session_liveness_checkpoint_queue
                    ORDER BY sequence DESC OFFSET 128
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE public.iam_session_liveness_checkpoint_queue")
            .await?;
        Ok(())
    }
}
