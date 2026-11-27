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
   principals, grant the IAM writer `SELECT`, `INSERT`, `UPDATE`, and `DELETE`
   on the epoch projection so its invoker-rights trigger can maintain it.
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
   that both are `ready` in `pg_subscription_rel` for `iam_projection`. Confirm
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
