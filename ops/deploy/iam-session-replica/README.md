# Bootstrap the IAM session replica on PostgreSQL 15

This prepares the database boundary for [ADR 0317](../../../docs/adr/0317-bounded-replica-session-admission.md).
Keep replica admission disabled during bootstrap, restore or downgrade. The
subscriber must start with empty tables in a trusted database, with a new
subscription and slot. This procedure is for distinct publisher and subscriber
clusters; the integration fixture uses separate databases in one cluster.

1. Apply IAM migrations to the authoritative writer. Configure `wal_level=logical`
   and enough replication slots and WAL senders for initial table copy and
   ongoing apply. Size and monitor WAL retention so a stopped subscriber
   cannot exhaust the writer's disk.
2. Provision `iam_session_replication` on the writer as a dedicated
   `LOGIN REPLICATION` role without superuser, role creation, ownership, or
   application role membership. Restrict its network access to the subscriber
   and keep its credential in trusted replication infrastructure. Run
   `psql -X -v ON_ERROR_STOP=1 -1 -f publisher.sql` on the writer as the
   projection table owner. If migrations and IAM runtime use different
   principals, run `psql -X -v ON_ERROR_STOP=1 -v iam_writer=<role> -1 -f iam-writer.sql`
   as the projection owner. These are the additional privileges for the
   invoker-rights projection trigger and checkpoint writer.
3. Provision `iam_session_admission` on the isolated subscriber as a dedicated
   `LOGIN` without replication, privileged memberships, ownership, or object
   creation privileges. Permit it to connect only to the subscriber database,
   with no route or credentials to the IAM writer. Run
   `psql -X -v ON_ERROR_STOP=1 -1 -f subscriber.sql` there as the trusted schema
   owner. The schema owner and subscription owner are never gateway identities.
4. As the trusted PostgreSQL 15 subscription operator, outside a transaction,
   create a single subscription for both relations. Provide the publisher
   endpoint and credentials in a server-side libpq service and password file
   readable only by the PostgreSQL server identity; use `sslmode=verify-full`
   with the deployment CA. For example:

   ```sql
   CREATE SUBSCRIPTION iam_projection
       CONNECTION 'service=iam_session_publisher'
       PUBLICATION iam_projection_publication
       WITH (copy_data = true);
   ```

5. Verify that the publication lists exactly the two projection relations and
   that both are `ready` in `pg_subscription_rel` for `iam_projection`. Install
   `readiness.sql` as the trusted subscription operator with a fresh operator
   UUID (`psql -X -v ON_ERROR_STOP=1 -v generation=<uuid> -1 -f readiness.sql`).
   This exposes readiness and generation without granting access to
   subscription connection information. Confirm
   that the gateway role can read the two relations but cannot mutate them,
   read unrelated application tables or connect to the writer. A completed
   copy only establishes bootstrap; admission also requires a fresh challenge
   observed through normal apply after readiness, as specified in the
   [admission contract](../../../docs/contracts/password-recovery-replica-admission.md).

PostgreSQL 15 [logical replication privileges](https://www.postgresql.org/docs/15/logical-replication-security.html)
require `SELECT` for initial copy and superuser to create a subscription.
Publication membership is not an access-control boundary for the replication
credential. Protect subscriber table ownership and DDL from untrusted roles.
For the [same-cluster fixture](https://www.postgresql.org/docs/15/sql-createsubscription.html#SQL-CREATESUBSCRIPTION-NOTES),
create a `pgoutput` slot first, then specify `create_slot = false`.

Retire the subscription and its slot before dropping the publisher projection.
Reseed after an unverifiable restore; do not skip transactions, reuse a stale
slot or keep pre-restore proof state.

## Configure a qualification environment

- Configure IAM's `session_checkpoint_token_env` to the name of a secret
  environment variable containing a dedicated checkpoint capability. It must
  differ from the IAM internal RPC token. Give only the admission worker this
  capability, pass it as `EXT_AUTHZ_CHECKPOINT_TOKEN`, and use the private,
  TLS-verified IAM writer URL in `EXT_AUTHZ_CHECKPOINT_URL` (path
  `/iam/internal/auth/session-liveness/checkpoint`). The marker route writes
  to the IAM primary used by recovery and returns `204` only after commit.
  For deployments with a private CA or client authentication, mount the
  corresponding PEM files and set `EXT_AUTHZ_CHECKPOINT_CA_FILE` and
  `EXT_AUTHZ_CHECKPOINT_IDENTITY_FILE` as applicable.
- Set `EXT_AUTHZ_REPLICA_DSN` to the subscriber-only SQL connection with
  `sslmode=verify-full`, using its CA and the `iam_session_admission` login.
  `EXT_AUTHZ_REPLICA_GENERATION` must match the UUID installed by the
  subscriber operator. Use a single trusted subscriber endpoint; never leave
  the generation unchanged while retargeting an endpoint to a restored copy.
- Configure the existing IAM liveness fallback and exact platform issuer.
  Use the private HTTPS URL for `/iam/internal/auth/session-liveness`, directly
  on the same fenced IAM authority. The fallback client accepts
  `EXT_AUTHZ_LIVENESS_CA_FILE` and `EXT_AUTHZ_LIVENESS_IDENTITY_FILE` for its
  private CA and client identity. Reserve up to
  40 fallback starts per second and eight outstanding calls per running
  gateway, plus one checkpoint per second per gateway. The worker rejects
  excess rather than queueing it. Checkpoint creation is separately bounded
  by IAM and retires markers beyond the last 128 in writer order. Validate
  that the configured gateway count and marker traffic preserve at least
  the proof window and the aggregate IAM capacity reservation.
  `EXT_AUTHZ_LIVENESS_TIMEOUT_MS` can further limit each fallback request
  below the 600 ms internal ceiling.
- Use the gateway's periodic `session admission totals` log to measure
  replica, authority and unavailable decision count deltas. It includes
  current proof age when known and no token or account labels.

Replica mode requires the Linux suspend-inclusive clock and a verified
bootstrap. Before enabling it, qualify the Envoy timeout and error mapping,
delayed-positive disposal, and the five-second cutoff using the deployed
admission paths.
