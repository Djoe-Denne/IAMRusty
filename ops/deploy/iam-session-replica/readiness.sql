-- Run once after creating iam_projection, as the trusted subscriber operator:
-- psql -X -v ON_ERROR_STOP=1 -v generation=<fresh-uuid> -1 -f readiness.sql
-- Use a different generation for every new or restored subscription.
CREATE TABLE public.iam_session_liveness_generation (
    singleton BOOLEAN PRIMARY KEY CHECK (singleton),
    generation UUID NOT NULL,
    subscription_oid OID NOT NULL
);

INSERT INTO public.iam_session_liveness_generation
VALUES (
    true,
    :'generation'::uuid,
    (SELECT oid FROM pg_catalog.pg_subscription
     WHERE subname = 'iam_projection'
       AND subdbid = (SELECT oid FROM pg_catalog.pg_database WHERE datname = current_database()))
);

CREATE VIEW public.iam_session_liveness_ready WITH (security_barrier = true) AS
SELECT g.generation AS configured_generation,
       md5(g.generation::text || ':' || s.oid::text || ':' ||
           extract(epoch FROM pg_catalog.pg_postmaster_start_time())::text)::uuid AS generation
FROM public.iam_session_liveness_generation g
JOIN pg_catalog.pg_subscription s ON s.oid = g.subscription_oid
WHERE s.subenabled
  AND s.subname = 'iam_projection'
  AND s.subpublications = ARRAY['iam_projection_publication']::text[]
  AND s.subdbid = (SELECT oid FROM pg_catalog.pg_database WHERE datname = current_database())
  AND (
      SELECT count(*) = 2 AND bool_and(
          r.srsubstate = 'r' AND r.srrelid IN (
              'public.iam_session_liveness_epochs'::regclass,
              'public.iam_session_liveness_checkpoints'::regclass
          ))
      FROM pg_catalog.pg_subscription_rel r WHERE r.srsubid = s.oid
  );

GRANT SELECT ON public.iam_session_liveness_ready TO iam_session_admission;
