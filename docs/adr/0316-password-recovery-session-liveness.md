# ADR-0316 — Durable authentication epochs make password recovery revoke prior platform access at every admission boundary

- Status: **Proposed**
- Reality: **Unimplemented**
- Date: 2026-11-24
- Deciders: pending product and architecture ratification; this draft does not ratify itself
- Scope: STORY-003.02; STORY-003 v3 E3 and STORY-010 v1 E2, limited to password recovery
- Supersedes: none
- Resolves: the `auth_version` / `session_version` / access deny-list question explicitly left open by ADR-0304 and ADR-0305
- Related: ADR-0302, ADR-0304, ADR-0305, ADR-0308, ADR-0310, ADR-0312, ADR-0314, ADR-0200–0202
- Evidence baseline: `b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f`; task revision `6912d9892b0845f2527c0ac9de088a7d82cfc11c`
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

1. Add the durable epoch and refresh-row epoch additively, defaulting existing accounts and rows to zero. No migration identifier is reserved by this draft.
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

## Product and architecture approvals required

1. **Availability tradeoff:** accept synchronous, fail-closed liveness on every protected platform request, including IAM/primary-database outage behavior.
2. **Legacy policy:** accept epoch-zero compatibility until the account's first post-cutover recovery reset, rather than forcing all users to sign in at deployment.
3. **In-flight boundary:** accept revocation at the admission/liveness-read linearization point, without cancellation of requests already admitted.
4. **Linked sign-in methods:** confirm that recovery invalidates earlier OAuth-issued platform sessions for the same account, while leaving external organization-managed identity sessions untouched.
5. **Authority topology:** confirm that the deployment has one authoritative epoch store per platform account. A multi-region asynchronous read topology would require a different design.
6. **Authenticated-change exclusion:** confirm that signed-in current-password change remains unchanged, including its existing access-token-until-expiry behavior.

## Validation status

This proposal is grounded in static inspection of the pinned baseline.
The [password reset suite](../../services/IAMRusty/tests/auth_password_reset.rs)
is an evidence candidate. The proposed direct, mesh, and concurrency oracles
remain unverified and require qualification after ratification.
