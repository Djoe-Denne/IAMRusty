# ADR-0316 — Durable authentication epochs make password recovery revoke prior platform access at every admission boundary

- Status: **Accepted**
- Reality: **Partial — IAM durable epoch, recovery/session writer, issuance and refresh slice qualified; shared direct/mesh admission enforcement pending**
- Date: 2026-11-24
- Deciders: product owner/user, 2026-11-24
- Scope: STORY-003.02; STORY-003 v3 E3 and STORY-010 v1 E2, limited to password recovery
- Supersedes: none
- Resolves: the `auth_version` / `session_version` / access deny-list question explicitly left open by ADR-0304 and ADR-0305
- Related: ADR-0302, ADR-0304, ADR-0305, ADR-0308, ADR-0310, ADR-0312, ADR-0314, ADR-0200–0202
- Evidence baseline: `b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f`; task revision `6912d9892b0845f2527c0ac9de088a7d82cfc11c`
- Implementation base: `f7b7489bc45cee7be2249652065dcad682d48a2d`
- Secondary contract: [Password recovery session liveness](../contracts/password-recovery-session-liveness.md)
- Shared issuance boundary: [ADR 0414](0414-oauth-provider-intents.md)

## Context

STORY-003 v3 E3 requires that, as soon as a reset from a recovery link succeeds, all earlier sessions lose authenticated access and cannot renew. STORY-010 v1 E2 adds that renewal cannot reopen a session ended by password recovery. STORY-003 E1 requires a reset request to leave sessions and the account usable; STORY-003 E2 requires confirmation without creating a session.

The current IAM writer changes the password, deletes every refresh row and deletes reset-token rows in one transaction. That is a sound renewal fence, but it does not revoke already-issued access JWTs. Those JWTs contain `sub`, `iss`, `aud`, `exp`, `iat`, `jti`, optional `org`, and no session or account-generation claim. Direct consumers validate them locally against JWKS; the opt-in mesh validates them in `ext-authz` and then forwards only `(iss, sub)`. Neither path consults durable session state.

The same writer currently serves unauthenticated recovery and authenticated current-password change. This proposal must not change the latter's behavior. Logout, browser binding, factor flows, and signed-in password-change policy remain outside this ADR.

## Decision

### 1. Durable authority: a monotonic platform-account authentication epoch

IAM owns a durable `auth_epoch` for each local platform account. It starts at zero, is never decremented or reused, and changes only when an unauthenticated recovery reset commits. The epoch is account-wide across devices and sign-in producers; it is not a per-browser or per-device identifier.

The canonical principal remains `(iss, sub)`. The epoch is consulted only after the issuer has been proved equal, byte for byte, to the configured platform issuer. An organization-managed issuer, an `org` claim, a `kid`, a URL suffix, or a matching UUID does not select the platform account. Storage on the local user row is an implementation detail guarded by that exact issuer check, not a flattening of the principal model in ADR-0305/0314.

### 2. Every newly issued platform access JWT carries the epoch

Add a required `auth_epoch` claim to newly issued platform access JWTs. The draft contract encodes it as a canonical decimal string carrying an unsigned 64-bit value, avoiding JSON-number precision ambiguity. Missing, malformed, negative, padded, or overflowing values are invalid on a newly issued token; only the explicit legacy rule below permits absence.

All sign-in producers must pass the same epoch used by their final persistence fence:

- email/password login (its current persistence fence is `SeaOrmTotpStore::issue_password_session`; only epoch plumbing changes, not factor semantics);
- verified registration completion;
- complete-user OAuth login;
- refresh rotation;
- any future platform sign-in producer using the shared session writer.

The OAuth domain service currently mints an auxiliary JWT that the application use case discards. The observable OAuth access token comes from `JwtTokenService::generate_access_token` and must carry the epoch. OAuth intent, transaction, cookie, PKCE, callback, link, and relink semantics are not redesigned here.

### 3. Reset success is the database commit, not the request or the HTTP response

A recovery reset is successful when one primary-writer transaction has committed all of the following:

1. lock the actual user row;
2. revalidate the expected password snapshot and the reset token's owner, hash, expiry, and unused state;
3. set the new password hash;
4. increment `auth_epoch` exactly once, with checked overflow;
5. delete all refresh rows for the account;
6. delete all reset-token rows for the account;
7. commit.

Only after commit may IAM return the existing confirmation message. The response contains no access token, refresh token, user object, or session. A lost response after commit does not undo the reset: a retry sees a consumed token, the new password works, and the prior epoch remains revoked.

Requesting or validating a reset does not change the password, epoch, refresh rows, account lock state, or access tokens. A failed confirmation leaves all of them unchanged.

### 4. Prior access is checked synchronously at every admission boundary

Cryptographic JWT validation remains necessary but is no longer sufficient for a platform access token. After signature, algorithm, `typ`, `kid`, issuer, audience, time, and trust validation, the admitting boundary must obtain an authoritative liveness decision for `(platform issuer, sub, auth_epoch)`:

- default direct mode: the shared RustyCog HTTP authentication hook checks before inserting an authenticated principal;
- opt-in mesh mode: `workers/ext-authz` checks before recreating `x-principal-iss` and `x-principal-sub`; the service trusts that gateway decision under ADR-0308's mTLS/SAN rule and does not duplicate the check;
- IAM's own authenticated routes use the same direct hook, with an in-process adapter allowed in the monolith;
- every deployed protected route must use the shared hook. A service-specific JWT decode or UUID-only path is not an alternative admission boundary.

The liveness read must use the authoritative primary, or an equivalent strongly consistent authority, and must not be served from a stale read replica. There is no positive cross-request cache, TTL, or revocation-bus grace period. JWKS caching remains unchanged and is not used as a session cache.

A decision is linearized at the authoritative liveness read. A request admitted before the reset commit may finish; the reset does not cancel work already admitted. Every request whose liveness read occurs after commit is denied. This is the proposed meaning of "immediate" and requires product approval if cancellation of in-flight work is expected.

### 5. Renewal is fenced by both the refresh row and the current epoch

Persist the epoch on new refresh rows. Existing rows migrate to epoch zero. Rotation must lock the user row and require all of the following in the same transaction:

- the old refresh row exists, is valid, unexpired, and owned by the user;
- the old row's epoch equals the user's current epoch;
- the replacement refresh row is inserted with that same epoch;
- the old row is removed;
- the transaction commits before any token pair is returned.

Access signing must occur outside the user lock, preserving ADR-0310's no-remote-operation-under-lock rule. An implementation may mint with the captured epoch and then perform a final epoch-and-token fence before commit. If the fence fails, no token pair is returned. If rotation commits before a concurrent reset, its epoch becomes stale at reset commit and its access token is denied even if the HTTP response arrives later. If reset commits first, rotation fails and cannot recreate a refresh row.

### 6. Legacy credentials have a bounded compatibility rule

Before the first post-cutover recovery reset, a platform access JWT without `auth_epoch` is treated as epoch zero and may continue until its normal expiry. Existing refresh rows are epoch zero and may rotate under the same fence.

The first successful recovery reset increments the account from zero to one. From that point, every platform access JWT without `auth_epoch` for that account is denied, even if it was issued before deployment. New tokens always carry the current epoch. This avoids a forced global sign-out at deployment while ensuring that no pre-feature session survives the account's first recovery reset.

A platform token with a present but malformed epoch is never treated as legacy. The historical `iamrusty` issuer is accepted only where the existing principal policy explicitly admits it in a bounded compatibility or isolated-test window; liveness never infers that alias from a URL shape.

### 7. Recovery and authenticated password change remain distinct operations

The implementation must give recovery and signed-in current-password change distinct session semantics, even if they share lower-level helpers:

- recovery increments `auth_epoch` and purges refresh rows;
- authenticated current-password change keeps its current behavior and does not increment `auth_epoch`.

The current authenticated flow already purges refresh rows; its already-issued access JWTs therefore remain valid until expiry. Preserving that behavior is an explicit scope constraint, not a claim that it satisfies STORY-003 E3. Any future change to signed-in password-change revocation requires a separate product decision.

### 8. JWKS, key revocation, and OpenFGA are not per-session revocation

This ADR does not revoke signing keys, publish an empty JWKS, alter key status, or delete OpenFGA tuples to implement password recovery. Those mechanisms have different authority and blast radius:

- JWKS/key revocation is trust-domain-wide and propagates through caches; it is not an account/session decision;
- OpenFGA is the authorization PDP, not the authentication-session authority, and projection/check behavior does not prove immediate AuthN revocation on every route;
- refresh-row deletion is necessary but only covers renewal.

A reset leaves other accounts, signing keys, organization identities, memberships, and permissions unchanged.

## Alternatives compared

### Durable epoch plus synchronous liveness check — selected

- Latency: adds one bounded authoritative read per protected platform request; no KMS call and no per-request JWKS fetch. No latency measurement is claimed.
- Availability: IAM and its authoritative database become a protected-request dependency; failure is fail-closed. This is the principal cost.
- Migration: additive epoch columns/claim, shared SDK admission hook, direct-service wiring, `ext-authz` wiring, and producer updates.
- Boundary coverage: direct, mesh, IAM itself, and monolith routing can all use the same decision. It is the only evaluated option that meets strict post-commit denial without a propagation window.

### Per-session registry plus synchronous check

- Latency and availability are similar to the selected option.
- Migration is larger: every access token needs a stable `sid`, every issuance must persist it, and cleanup/retention become mandatory.
- It supports future per-session revocation, but STORY-003.02 needs account-wide recovery revocation only. The extra capability does not justify the migration now.

### Epoch or session denylist with push invalidation and local cache

- Latency and steady-state availability are better.
- A cache or bus introduces a propagation window, and fail-open behavior violates E3. A zero-TTL negative-only cache still needs an authoritative initial decision.
- Migration adds a revocation distribution system and consistency proofs. Rejected for strict immediacy.

### `jti` denylist

- Current access `jti` values are not persisted, so IAM cannot enumerate all outstanding tokens to revoke them.
- Persisting every access token adds unbounded state and cleanup, while still requiring synchronous checks for immediacy.
- It offers no advantage over an account epoch for an account-wide reset.

### Opaque access tokens with IAM introspection

- Introspection can provide strong liveness.
- It replaces the RS256/JWKS consumer contract, changes every direct and mesh path, and has a larger compatibility and operational blast radius. Rejected as incompatible with the current architecture.

### Shorter access TTL or current refresh purge only

- Latency, availability, and migration are excellent.
- Prior access remains usable until expiry, so STORY-003 E3 is not met. Rejected.

### Revoke or rotate the platform signing key / publish an empty JWKS

- Propagation is cache-bound and affects every account under that key, not just the recovered account.
- It confuses key trust with session revocation and can cause a platform-wide outage. Rejected.

### Revoke OpenFGA grants or delete tuples

- This is AuthZ, is projection-dependent, and does not cover non-FGA routes or renewal.
- It cannot serve as the immediate AuthN oracle. Rejected.

### Enforce only at the mesh gateway

- It would leave the default direct mode and directly reachable service ports unprotected.
- Making mesh mandatory is a topology and availability change beyond this lot. Rejected; both current modes must be covered.

## Compatibility and migration

1. Add the durable epoch and refresh-row epoch additively, defaulting existing accounts and rows to zero. The implemented migration is `m20261124_000001_auth_epoch`; its downgrade refuses to drop an advanced epoch.
2. Deploy IAM issuance and the authoritative liveness endpoint. New platform tokens and refresh rows carry the epoch.
3. Deploy the shared direct admission hook to IAM, Hive, Manifesto, Telegraph, and the monolith. OAuthConnect has no Bearer routes; Lazaret currently exposes workload/session routes rather than IAM HumanAccount Bearer routes. Re-inventory before cutover rather than assuming this remains true.
4. Add the same liveness decision to `ext-authz` for opt-in mesh deployments. Missing liveness configuration while enforcement is enabled is a boot failure.
5. Enable enforcement only after the complete direct/mesh route inventory and all producer fences are verified. During a partial rollout, this ADR's E3 guarantee is not yet claimed.
6. Keep legacy access tokens without an epoch admissible only at account epoch zero. Do not decrement epochs or reissue old tokens on rollback. Disabling enforcement after a recovery reset reopens a known security gap and requires explicit incident approval.

The historical statement that there was no production data does not establish the state of a future deployment. The implementation plan must inventory the actual database, outstanding tokens, refresh rows, replicas, and mesh exposure before migration.

## Failure behavior

- Invalid, expired, used, wrong-owner, or stale reset proof: no commit, no epoch change, no purge, no success response.
- Epoch overflow, database error, purge failure, or commit failure: rollback password, epoch, refresh rows, and reset tokens together.
- Liveness store unavailable, timed out, malformed, or inconsistent: deny required protected access; never fall back to JWT-only validation. The proposed external status is 503 for infrastructure failure and 401 for a revoked credential.
- Optional-auth routes must not acquire an authenticated context from an unknown decision. The draft keeps the existing behavior of rejecting a presented invalid token rather than silently converting it to anonymous.
- Signing-provider failure prevents new login/refresh tokens but does not bypass liveness for tokens already issued.
- JWKS freshness/outage and OpenFGA behavior remain governed by ADR-0304/0308/0302/0314 and are not weakened or repurposed.
- Organization-managed principals are outside the local password epoch. They retain their existing trust and authorization behavior; they are not globally banned or mapped to a platform UUID.

## Consequences

- Password recovery now has an immediate, observable AuthN effect at every covered admission boundary, not merely a renewal effect.
- Protected-route availability now depends on a strongly consistent IAM liveness read. Capacity, timeout, pooling, and outage behavior require measurement and operational approval.
- The access-token format and shared SDK admission path change. Existing clients can carry the new opaque claim without change, but old tokens need the explicit legacy rule.
- Reset and authenticated password change can no longer be treated as semantically identical writer calls.
- The design does not add logout, browser binding, factor flows, per-session revocation, or an OAuth redesign.

## Product approval record

The product owner approved the selected account-wide epoch design for STORY-003.03 backend work:

1. a successful recovery reset advances the epoch at primary commit;
2. every later protected admission or renewal using an earlier platform epoch is denied, including OAuth-issued platform sessions;
3. requests admitted before commit may finish; in-flight cancellation is not required;
4. authority unavailability fails closed; a verified JWT alone is insufficient;
5. signed-in current-password change keeps its existing behavior;
6. legacy access without an epoch remains admissible only at account epoch zero.

This approval does not select a production topology, authorize production access, approve indefinite SMTP suppression, or authorize RustyCog edits, publication, or a gitlink bump.

Before enforcement, the authoritative-store topology, bounded timeout/capacity budget, complete route inventory, trusted workload path and concrete shared SDK/public seam still require operational and implementation qualification.

## Validation status

The design baseline remains `b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f`. A native red before implementation recorded an unexpired stale access JWT receiving `200` instead of `401` on a protected direct IAM route.

The IAM slice was qualified on base `f7b7489bc45cee7be2249652065dcad682d48a2d`:

- formatting, integration-target type-check and final unit-test compilation passed;
- `iam-domain` passed 89/89 and `iam-infra` passed 155/155;
- the HTTP/PostgreSQL reset binary passed 27 tests, retained exactly the known stale-access `200`-vs-`401` red, and ignored one opt-in test;
- the migration binary passed 8/8;
- `session_writer_concurrency` passed 2/2 in campaign
  `it-e3f21472774445408e02e027f22a33cd`; one fixture was created and removed,
  with zero campaign-owned running and the three pre-existing Kind nodes unchanged;
- complete direct Clippy production/tests lanes passed with valid JSONL reports
  and no finding attributable to an added line after the test-only correction;
- all campaign-owned fixtures were removed, with pre-existing registered resources left untouched.

The later test-only Clippy correction left every runtime-relevant blob unchanged relative to the qualified HTTP/PostgreSQL envelope; the production prefix of the refresh service is byte-identical and the added TLS target is outside that campaign.

The direct stale-access test remains intentionally red until a published RustyCog liveness hook and IAM reader wiring exist. Authority-outage `503`, optional-auth no-downgrade, legacy no-claim admission before/after the first reset, real mesh Check behavior, exact outage mapping, trusted workload path, boundary inventory and deterministic late-response rotation ordering remain unverified. STORY-003.03 therefore remains open.
