# ADR-0317 — Use verified PostgreSQL replicas for bounded recovery revocation

- Status: **Proposed**
- Reality: **Unimplemented**
- Date: 2026-11-27
- Scope: STORY-003 v4 E3 and STORY-003.03.01
- Proposed amendment: the mesh admission path and strict zero-delay access
  cutoff in [ADR 0316](0316-password-recovery-session-liveness.md).
- Contract: [Bounded replica admission](../contracts/password-recovery-replica-admission.md)

## Context

[PR #57](https://github.com/Djoe-Denne/IAMRusty/pull/57) shipped a durable
per-account epoch, an atomic password reset and refresh purge, and an optional
synchronous IAM check in the gateway. The synchronous decision forces both IAM
and its primary to handle every authenticated platform admission at the gateway
when enabled. STORY-003 v4 instead requires immediate renewal revocation and
bounds prior access to five seconds from the reset commit, including failures.

## Proposed decision

IAM remains the authority for session state and reset ordering. Replicate a
minimal session-state projection and a freshness marker through one ordered
PostgreSQL logical subscription. The gateway uses the subscriber for normal
platform admissions, with no per-admission call to IAM or its primary. Use an
explicit checkpoint from the primary and prove the checkpoint was applied
before allowing from the replica. The checkpoint expires, including when the
subscriber still answers queries. A bounded IAM fallback is permitted; lacking
a trustworthy decision, refuse admission temporarily.

Preserve the epoch and legacy claim format, issuer boundary, atomic writer,
refresh rotation fence, direct authoritative paths, and the gateway-to-service
trust boundary of ADR 0316. This proposal creates no per-token deny list,
new signing-key revocation mode, or new authorization policy.

## Alternatives

- **Synchronous IAM HTTP:** already available and authoritative, useful for
  direct paths and fallback, but keeps IAM and the primary on the normal mesh
  path.
- **Read the IAM primary through a restricted SQL interface:** removes the
  application hop and preserves the strict cutoff; the primary still handles
  every admission. A physical read replica can isolate reads but would copy
  unrelated IAM data and would still require proof of replay freshness.
- **SQS plus Redis:** viable as a projection mechanism, but an outbox and
  idempotent consumer do not alone prove contiguous application, bootstrap, or
  recovery after missed data. PostgreSQL already provides ordered transactional
  delivery within a subscription and avoids a broker and custom materializer
  for this slice.
- **Synchronous replica apply or all-gateway acknowledgment:** stronger
  visibility can be established under the right topology, at the cost of
  making the reset path wait for the serving read topology. The approved
  product window allows asynchronous application with fail-closed admission.

## Consequences

In normal operation the worker still does a bounded subscriber lookup per
admission. This proposal does not assert latency, throughput, or failover
performance before measurement. A stale subscriber shifts bounded traffic to
IAM and can cause temporary authenticated-request failures. Correctness relies
on a single fenced IAM writer lineage, controlled subscriber lifecycle, and no
out-of-band change or skipped transaction in the projection. Workload access,
configuration and deployment qualification are explicit rollout gates. Leaving
the former live-check configuration absent is not a valid rollout.

Deployment work targets the existing opt-in mesh test environment. No cloud
provisioning, production activation or modification to published ADR status is
implied by this proposal.
