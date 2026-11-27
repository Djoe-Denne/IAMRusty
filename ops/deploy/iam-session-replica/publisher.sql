-- Run on the IAM writer, after the projection migration, as the table owner.
-- iam_session_replication is a dedicated infrastructure-managed LOGIN REPLICATION role.
CREATE PUBLICATION iam_projection_publication FOR TABLE
    public.iam_session_liveness_epochs,
    public.iam_session_liveness_checkpoints;

GRANT USAGE ON SCHEMA public TO iam_session_replication;
GRANT SELECT ON
    public.iam_session_liveness_epochs,
    public.iam_session_liveness_checkpoints
TO iam_session_replication;
