-- After IAM migrations; run as the projection owner.
-- psql -X -v ON_ERROR_STOP=1 -v iam_writer=<role> -1 -f iam-writer.sql
GRANT USAGE ON SCHEMA public TO :"iam_writer";
GRANT SELECT, INSERT, UPDATE, DELETE
    ON public.iam_session_liveness_epochs TO :"iam_writer";
GRANT SELECT, INSERT, DELETE
    ON public.iam_session_liveness_checkpoints TO :"iam_writer";
GRANT SELECT, INSERT
    ON public.iam_session_liveness_checkpoint_queue TO :"iam_writer";
GRANT USAGE
    ON SEQUENCE public.iam_session_liveness_checkpoint_queue_sequence_seq TO :"iam_writer";
