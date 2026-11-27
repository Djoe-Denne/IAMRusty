# Password recovery: bounded admission from a PostgreSQL replica

- Status: **Proposed / Unimplemented**
- Product: STORY-003 v4 E3, STORY-003.03.01, and STORY-010 E2
- Evidence baseline: `51e789ac4a77e188e922271fc7ec6076ebd66628`
- Existing invariants and evidence: [ADR 0316](../adr/0316-password-recovery-session-liveness.md) and [its implementation contract](password-recovery-session-liveness.md)

## 1. Observable guarantee

`T` is the IAM primary commit of a successful, link-based recovery reset. Requests
to send an email, validate a link, or confirm with invalid proof do not change
the session epoch. Confirmation creates no session.

An earlier refresh credential cannot renew from `T`. A rotation that commits
before `T` may return its result later, but the replacement refresh credential
has already been removed and its access JWT is subject to the same cutoff.
An unexpired earlier platform access JWT must not grant a new protected
admission at or after `T + 5 s`, even during replication or fallback failure.
An admission decision made earlier may finish. The five seconds are a maximum,
not a percentile or an availability promise. Expiry and cryptographic checks
still apply independently.

## 2. Admission boundaries

- **IAM direct and IAM routes nested in the monolith:** retain the in-process
  primary checker and refuse expired admission attempts before forwarding.
- **Hive, Manifesto, and Telegraph direct, including their nested routers:**
  retain the shared authoritative HTTP checker for supported independent
  ingress. When rollout declares these paths protected, configure that checker
  or prevent platform bearer access to the direct path. JWT-only admission is
  not a supported alternative.
- **Envoy mesh for IAM, Hive, Manifesto, and Telegraph:** workers make the
  admission decision before forwarding identity. Services trust only the
  established gateway mTLS/SAN boundary and do not recheck admitted work.
  Route matching must preserve protected/default-deny behavior, strip client
  principal headers and keep worker-only endpoints off public ingress.
- **Optional authentication:** on supported routes, no credential keeps the
  existing guest behavior. A presented credential is subject to the same
  bound and must never turn into anonymous admission on rejection or error.
  This applies wherever optional admission is exposed; it does not turn an
  existing mesh route that requires credentials into a public route.
- **Sensitive actions:** the five-second ceiling never weakens any existing
  current-password, factor, role, or service-origin gate. No new step-up rule
  or per-request IAM dependency on the healthy mesh path is introduced by this
  proposal. Before rollout, verify those gates on provider-token retrieval and
  revocation, TOTP enrollment, email verification, password change, and
  provider link/relink. Record and resolve any route found to require a
  stricter cutoff before that route is admitted by the replica.

Source inventory at the baseline (builder registrations, not method counts):

- [IAM router](../../services/IAMRusty/http/src/lib.rs): 12 required-auth
  registrations (including the combined `GET`/`PATCH /api/me`); profiles,
  factors, email, provider link/relink, signed-in password change and internal
  provider routes that also keep their independent workload checks. Public
  reset/refresh and the workload-only liveness endpoint are separate.
- [Hive router](../../services/Hive/http/src/lib.rs): 21 required-auth
  registrations; organizations, membership and roles, invitations, sync,
  external links and signers. Organization search is optional-auth.
- [Manifesto router](../../services/Manifesto/http/src/lib.rs): 22 required-auth
  registrations for projects, binding consent, components, membership and
  permissions; five optional-auth project/component reads. The additional
  grant-snapshot router uses a [dedicated workload-only HS256 extractor](../../services/Manifesto/setup/src/app.rs)
  and does not consume platform RS256 JWTs. Runtime-origin endpoints have a
  separate workload boundary.
- [Telegraph router](../../services/Telegraph/http/src/lib.rs): three
  required-auth notification registrations.
- [Monolith composition](../../runtime/monolith/src/routes.rs) nests the same
  routers. [Compose Envoy](../../ops/deploy/mesh/envoy.yaml) and
  [Kind Envoy](../../ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml) carry
  the four protected prefixes with narrowly defined exceptions. OAuthConnect
  has no user bearer routes; Lazaret's runtime sessions are separate. Verify
  these exclusions and the deployed route inventory before enabling rollout.

## 3. Replica admission proof

Use a PostgreSQL 15 logical subscription for the platform account projection
and its freshness marker, with ordinary ordered apply. The projection carries
only the platform subject and the current canonical unsigned `auth_epoch`.
Use a dedicated source relation maintained by PostgreSQL from changes to the
existing IAM account row in the same transaction, including create, epoch
advance and delete. The migration initializes epoch-zero as well as advanced
accounts; the subscriber must start from an empty, trusted bootstrap. A missing
account must not be mistaken for an epoch-zero account. A failed recovery
changes neither copy. A publication column list on `users` would avoid the
extra relation, but PostgreSQL 15
[does not make that list a security boundary](https://www.postgresql.org/docs/15/logical-replication-col-lists.html).
Keep replication credentials in trusted infrastructure and verify the source
role and publication restrictions separately. The gateway has read-only
access to the two subscriber relations and no IAM SQL credential.

A worker proves freshness using a unique challenge through a bounded
workload-only operation on the fenced IAM primary. The worker records a local
elapsed-time anchor **before** sending the challenge. The operation commits a
marker in the same publication and subscription as the session projection.
The proposed boundary is `POST /iam/internal/auth/session-liveness/checkpoint`:
accept one UUID challenge from an authorized admission workload and return
`204` only after commit; response receipt is not a replication confirmation.
Reject malformed inputs and unauthorized callers; unavailable or saturated
probe capacity does not renew the proof. Expose no account data from this route.
The worker may qualify its replica only after observing the exact fresh
challenge applied, with the expected source and subscription generation.
Probe about once per second initially; neither a periodic tick, standby status,
nor receipt of the IAM response alone can renew the proof.
Only one challenge per worker may be in flight. Keep applied markers long
enough to verify an unexpired proof, then remove them with a bounded retention
policy. Challenge writes have a separate capped background budget; requests
must not trigger probe writes. Use TLS and a dedicated workload capability
with no user-session or general SQL privileges.

Any reset committed before that anchor precedes the challenge in the source
commit stream. A trusted subscriber that has applied the challenge therefore
includes that reset. A reset not reflected in a permitted decision must have
committed after the anchor. A proof's age is measured from the challenge start,
not when its marker was first observed. The initial target envelope is
**3 s maximum proof age + 1 s maximum end-to-end Check delivery + 1 s safety
headroom**. At the proof horizon, do not allow from the replica. For any
qualified configuration, proof age plus bounded permission-delivery delay plus
elapsed-time uncertainty must be at most **5 s**. Use a clock that includes
suspension, or invalidate proof before serving after suspension.
[Rust `Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html)
does not portably promise suspend-inclusive time. Discard all local proof
state on restart; an IAM call or subscriber query completing late cannot
extend proof validity.

For each replica decision, read the proof marker and epoch in the **same
subscriber snapshot**, bound to the same subscriber generation and endpoint.
Matching a stored marker from another connection or another replica does not
qualify an older snapshot. After all awaits, recheck the proof age, JWT time
validity and current JWKS/key authorization before allowing. Do not reuse a
replica decision across requests. Envoy must discard an allow that cannot be
consumed inside the Check delivery budget; verify the delayed-positive case
on the deployed filter. A configured timeout alone is not evidence that this
is true. Apply the same principle to delayed positive direct or fallback
decisions so a late response cannot grant access beyond the product deadline.
An older JWT epoch than the valid replicated epoch is a definitive denial.
A newer epoch or absent account may reflect a recent login or creation not
yet applied; it cannot grant access from the replica. In that case the
worker may use the same bounded fallback to distinguish lag from revocation
or a missing account. A malformed JWT epoch is never legacy; a verified
token with an absent claim may match only an existing epoch-zero account.
Only the exact configured platform issuer triggers this logic. Non-platform
issuers keep their existing trust and authorization rules and are never
identified by the subject alone.

Do not admit from a new, restored or replaced subscriber until both required
relations are verified `ready` in that subscription. Send a **new** challenge
after that readiness check, and observe it through ordinary apply. A marker
copied during initialization is never proof. Bind readiness, challenge and
request reads to the same generation; do not carry a proof across pool
retargeting. Treat partial table sync, generation changes, non-authoritative
publisher/failover, stale rollback, or missing marker identity as untrusted.
The supported procedure forbids local projection edits, skipping transactions,
and independently restoring tables while retaining a qualifying marker;
invalidate admission before such maintenance and reseed through verified
initialization. An unverified bootstrap with the initial copy disabled is not
allowed. A new challenge and readiness catalogs cannot repair skipped history.
A paused subscriber and a reachable stale database are
different from a revoked account. Keep status/health signals diagnostic.
Source failover must preserve committed epochs; a source restored to an
earlier state must not serve old credentials under the same trust lineage.
Subscriber and publisher failover cannot be enabled merely by retaining a
hostname. A changed or unverifiable lineage invalidates replica proofs;
fallback may still run only against the fenced, non-regressing IAM authority.

PostgreSQL references: [logical replication and transaction order](https://www.postgresql.org/docs/15/logical-replication.html),
[per-table synchronization state](https://www.postgresql.org/docs/15/catalog-pg-subscription-rel.html),
and [replication role boundaries](https://www.postgresql.org/docs/15/logical-replication-security.html).

## 4. Bounded authoritative fallback

Use the existing workload-authenticated IAM liveness endpoint only when the
replica cannot produce a trustworthy decision for that query. Connect over
the internal workload path, without a recursive call through Envoy admission.
Cap in-flight calls, outstanding coalescing keys and call rate per gateway,
and qualify the aggregate budget for the permitted gateway
replica count against a separate IAM capacity reservation. No unbounded queue,
retries or bypass. Unused or exhausted fallback capacity must never turn into
an allow. Deployment owns the rate, concurrency and timeout values; evidence
must demonstrate the reservation and saturating behavior before activation.
The replica read and any fallback together must fit inside the same Check
deadline, including transport and response delivery.

Concurrent checks may share the same authoritative account-state read by
exact `(iss, sub)`, but must compare each caller's verified claim to that state.
Never share a Boolean allow across different epochs; the existing HTTP
request-specific response may be coalesced only for identical queries unless
the adapter demonstrably extracts coherent account state. In the initial slice
do not retain positive fallback decisions or write fallback data into the
replica. Anchor the fallback read at the local start and require a fresh
decision before allowing; a late response is temporary unavailability.

Apply the limit before dispatch; timeouts release concurrency capacity.
Do not consume IAM capacity for an invalid signature or a definite older-epoch
denial. For optional auth, a presented but unverifiable credential never
becomes a guest request.

Return the existing authentication rejection for a definitive stale token
(`401` on direct admission). Use temporary unavailability (`503`) when neither
source can decide; the mesh must forward no principal and configure Envoy
error handling accordingly, with `failure_mode_allow` disabled. In Envoy 1.31.2,
an external authorization `5xx` is an error path governed by
[`status_on_error`](https://www.envoyproxy.io/docs/envoy/v1.31.2/api-v3/extensions/filters/http/ext_authz/v3/ext_authz.proto).
Expose decision-source counts for replica, fallback and unavailable outcomes,
and proof-age/replication diagnostics without token or account labels.

At the baseline, [`ext-authz` checks JWKS before awaiting liveness and maps
all liveness errors to `403`](../../workers/ext-authz/src/check.rs). The
Compose and Kind Envoy filters both configure a five-second Check timeout.
Those are required changes for the proposed mesh behavior, not qualified
deployment settings.

## 5. Minimum TDD evidence

| Stage | First decisive failing assertion | Evidence required after implementation |
| --- | --- | --- |
| IAM and replica integration | A committed reset does not advance the epoch on a real PG15 subscriber through the proposed publication. | Bootstrap includes existing accounts; a committed reset advances the replica, a rolled-back one does not, and a marker cannot qualify an unapplied or incompletely initialized replica. |
| Gateway unit tests, controlled clock | A replica proof at the maximum age does not trigger a bounded authoritative fallback or temporary refusal. | Exact age boundary, delayed results, positive and negative admissions, stale/restored state, key freshness after awaits, same-account request sharing, contention and budget exhaustion; no IAM lookup on the healthy replica path. |
| Deployed nominal E2E | A protected downstream request cannot be attributed to replica admission with no per-admission IAM calls. | Positive before reset, post-reset denial on every gateway replica within the bound, explicit new login succeeds, and path-specific counters prove the source of each verdict. A positive Check response delayed past the delivery deadline never reaches a protected handler. |
| Deployed degraded E2E | The subscriber stays reachable while application is suspended; a stale verdict can still authorize or there is no verified fallback path. | Independent evidence that the old epoch is still in the replica, capped fallback or temporary refusal by the deadline, no forwarded principal on refusal, and resumption after catch-up. |

For elapsed time use a lower bound on commit, such as a test-owned database
barrier or the timestamp before sending confirmation; counting only from
receipt of the success response could hide a late rejection. Prove the
boundary deterministically in unit tests and keep any E2E measurement that
cannot distinguish the deadline inconclusive. A local infrastructure failure,
timeout before exercising the expected assertion, or a green HTTP status
without decision provenance is not TDD evidence.

The inherited oracles for legacy tokens, non-platform issuer isolation,
optional-auth, real trusted workload paths, and both refresh/reset HTTP
orderings remain in scope. Reuse evidence from [PR #57](https://github.com/Djoe-Denne/IAMRusty/pull/57)
for direct IAM revocation, SDK fixtures and the gateway stand-in. The
[green CI run](https://github.com/Djoe-Denne/IAMRusty/actions/runs/45883865200)
does not qualify a PostgreSQL subscriber or a deployed replica-backed mesh.
