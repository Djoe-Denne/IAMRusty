-- Run on a new, isolated subscriber database as its trusted schema owner.
-- iam_session_admission is a dedicated infrastructure-managed read-only login.
REVOKE CREATE ON SCHEMA public FROM PUBLIC;

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

GRANT USAGE ON SCHEMA public TO iam_session_admission;
GRANT SELECT ON
    public.iam_session_liveness_epochs,
    public.iam_session_liveness_checkpoints
TO iam_session_admission;
