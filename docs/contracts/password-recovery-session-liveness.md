# Password recovery session-liveness contract

- Status: **Accepted / Partially implemented**
- Date: 2026-11-24
- Decision authority: [ADR 0316](../adr/0316-password-recovery-session-liveness.md)
- Scope: STORY-003 E3 and STORY-010 E2, password-recovery branch only
- Baseline: `b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f`
- Normative language: **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are used intentionally.

## 1. Scope and non-scope

This contract defines when a successful recovery reset takes effect, how prior access and renewal are denied, how direct and mesh admission obtain the same decision, and which observations distinguish a reset from a reset request.

It does not define logout, browser binding, factor enrollment or recovery, signed-in password-change policy, reset-email rate limiting, OAuth intent transitions, organization-identity issuance, key rotation, OpenFGA schema changes, or a new revocation infrastructure.

## 2. State and token model

### 2.1 Durable state

IAM MUST maintain a monotonic `auth_epoch` for each local platform account:

- initial value: zero;
- type: unsigned 64-bit;
- mutation: increment exactly once in the successful unauthenticated recovery transaction;
- prohibited mutations: decrement, reset, reuse, or change on reset request/validation/failure;
- read authority for admission: the primary writer, or an equivalent strongly consistent store;
- stale replica reads: prohibited for liveness decisions.

New refresh rows MUST persist the epoch with which they were issued. Existing refresh rows MAY migrate to epoch zero. A refresh row whose epoch differs from the account's current epoch MUST NOT rotate.

The physical epoch column MAY live on the local user row because the current session writer already locks that row. This does not change the canonical principal: the liveness authority MUST first prove the exact configured platform issuer and MUST NOT apply the epoch to an organization issuer or to a UUID alone.

### 2.2 Access-token claim

Every newly issued platform access JWT MUST include:

```json
{
  "auth_epoch": "0"
}
```

`auth_epoch` is a canonical decimal string for an unsigned 64-bit integer:

- digits `0`–`9` only;
- no sign, whitespace, leading zeroes except the single digit `0`, fraction, or exponent;
- no value above `u64::MAX`;
- exact round-trip between IAM, direct consumers, and `ext-authz`.

The claim is opaque to end-user clients. Existing `sub`, `iss`, `aud`, `exp`, `iat`, `jti`, header `typ`, and `kid` requirements remain unchanged.

A platform token with a present but invalid `auth_epoch` MUST be rejected as an invalid token. It MUST NOT be treated as legacy.

### 2.3 Legacy rule

A platform access JWT without `auth_epoch` MAY be admitted only while the account's current epoch is zero. The first successful post-cutover recovery reset changes the account to epoch one; from then on, every platform token without the claim for that account MUST be denied.

A legacy refresh row at epoch zero MAY rotate before the first reset, producing a replacement row and access token at epoch zero. After a reset, no pre-reset row may survive or renew.

The historical `iamrusty` issuer is not inferred from string shape. It is eligible only where the existing principal policy explicitly admits it in a bounded compatibility or isolated-test window.

## 3. Authoritative liveness interface

The following logical interface is accepted. The shared SDK/public wiring and
IAM workload route are implemented; integrated qualification and rollout gates
remain.

```text
POST /iam/internal/auth/session-liveness
```

The caller MUST be an authenticated platform workload using the existing internal service credential/workload boundary. An end-user bearer alone is not sufficient authority to ask for or trust this decision. The request MUST NOT contain the raw access token, password, reset token, signing material, or cookies.

Request:

```json
{
  "issuer": "https://platform.example/iam",
  "subject": "2d98bfaf-5fcf-4d0d-95f9-bafc6125fb2f",
  "auth_epoch": "0"
}
```

`auth_epoch` is optional only to represent a verified legacy token. A direct caller or `ext-authz` MUST have already validated the JWT before calling. A mesh-mode service does not call this endpoint because `ext-authz` performs the check on its behalf.

Successful response decisions:

```json
{"decision":"live","current_epoch":"0"}
```

```json
{"decision":"revoked","current_epoch":"1"}
```

```json
{"decision":"not_applicable"}
```

Rules:

- `live`: exact platform issuer, account exists, and token epoch equals the current epoch.
- `revoked`: exact platform issuer and the token epoch is absent after the first reset, differs from the current epoch, or the platform account no longer exists; `current_epoch` is omitted when no account exists;
- `not_applicable`: the issuer is a recognized non-platform authority. No platform account epoch is consulted and no organization identity is banned.
- A missing epoch is `live` only at current epoch zero.
- A malformed request is `400`; an unauthorized service caller is `403`; an unavailable or inconsistent authoritative store is `503`.
- Responses MUST carry `Cache-Control: no-store`.
- The endpoint MUST NOT log raw tokens or treat a caller-supplied issuer as proof of trust. Caller authentication and exact issuer selection are separate controls.

The direct client and `ext-authz` MUST use bounded connection and total deadlines. The exact deadline is a deployment setting to be approved and measured; no timeout, retry, or error may produce an allow decision. Retrying a failed read is safe only if every attempt still reads authoritative state and failure remains fail-closed.

## 4. Admission behavior

### 4.1 Common order

For a platform Bearer token, every admission path MUST apply this order:

1. remove inherited identity extensions and untrusted `X-Principal-*` headers;
2. validate algorithm, header parameters, `kid`, signature, `typ`, issuer trust, audience, `exp`, `iat`, optional `nbf`, and UUID `sub`;
3. preserve the exact `(iss, sub)` principal;
4. determine whether `iss` is the exact configured platform issuer;
5. for a platform issuer, parse `auth_epoch` and obtain the authoritative liveness decision;
6. only then insert an authenticated principal or recreate mesh principal headers.

A valid signature or a cached JWKS document MUST NOT bypass step 5.

### 4.2 Default direct mode

The shared RustyCog HTTP authentication hook MUST perform the liveness check after cryptographic extraction and before installing `JwtPrincipal` in request extensions. The logical seam MAY be named `SessionLivenessChecker`; the public name is secondary to the behavior.

Required-auth routes:

- `live` or `not_applicable`: continue to the existing authorization checks;
- `revoked` or malformed platform epoch: `401`, with no principal installed;
- liveness infrastructure failure: `503`, with no principal installed and no JWT-only fallback.

Optional-auth routes:

- a missing token continues on the existing anonymous path;
- a presented malformed, cryptographically invalid, or revoked token remains an authentication error, preserving current behavior;
- a liveness infrastructure failure MUST NOT create an authenticated context; the draft returns `503` rather than silently changing a presented credential into anonymity;
- a `not_applicable` organization principal follows the existing issuer-aware authorization rules.

The same hook MUST cover IAM's own `.authenticated()` routes. In the monolith, an in-process adapter MAY avoid a network hop, but it MUST read the same authoritative epoch and MUST NOT be more permissive than the HTTP adapter.

### 4.3 Opt-in mesh mode

`workers/ext-authz` MUST perform the liveness check after JWT/JWKS validation and before returning `200` with recreated `x-principal-iss` / `x-principal-sub` headers.

- `live` or `not_applicable`: recreate only the existing principal headers;
- `revoked`, malformed epoch, or liveness failure: deny and create no principal headers;
- missing liveness configuration while enforcement is enabled: fail closed at boot;
- the mesh service MUST NOT add a positive liveness cache or reuse JWKS freshness as session freshness.

A service configured with `auth.mesh.trusted_gateway_san` MUST continue to accept principals only from the gateway mTLS peer with the exact configured SAN, per ADR-0308. It MUST NOT repeat the IAM liveness call in mesh mode; the gateway decision is the admission decision. Direct access to a service port with forged headers remains rejected.

### 4.4 Boundary inventory gate

Before enforcement is enabled, the implementation MUST inventory and test every deployed protected path that accepts a platform user session. At this baseline the known paths are:

- direct RustyCog authentication in IAM, Hive, Manifesto, and Telegraph;
- the same routes when nested in the monolith;
- opt-in `ext-authz` for the `/iam/`, `/hive/`, `/telegraph/`, and `/manifesto/` mesh prefixes;
- optional-auth public reads that can elevate behavior when a principal is present.

OAuthConnect explicitly has no Bearer routes. Lazaret currently has no IAM `.authenticated()` HumanAccount route and uses a distinct workload/runtime-session model. These are baseline observations, not permission to omit a future route from the inventory.

Public signup, login, refresh, reset request/validate/confirm, OAuth start/callback, JWKS, and probes remain outside user-session admission. Refresh and reset confirm enforce their own state rules below. Internal provider-token routes retain their separate service-credential gates.

## 5. Issuance and renewal semantics

### 5.1 Producer fence

Every platform access/refresh issuance MUST use an epoch captured from the same account snapshot that the final persistence writer validates. The writer MUST lock the user row and reject issuance if either the expected credential snapshot or epoch changed before commit.

This applies to:

- `AuthService::complete_login` for email/password; its actual persistence fence is `SeaOrmTotpStore::issue_password_session`, which MUST gain the epoch check while preserving all factor behavior;
- `RegistrationServiceImpl::complete_registration` for verified registration;
- `OAuthUseCaseImpl::oauth_login` for complete users, and any new provider Signup confirmation in [the OAuth intent contract](oauth-provider-intents.md);
- `RefreshTokenServiceImpl::refresh_token`;
- future producers using `AuthenticationSessionWriter`.

The access token MAY be signed before the final writer fence, but no token pair may be returned unless the corresponding refresh/session write commits. Remote signing MUST NOT occur while holding the user lock.

### 5.2 Refresh rotation

Given old refresh row `R` at epoch `E` and current account epoch `C`:

- if `C != E`, deny with `401`;
- if the old row is missing, invalid, expired, or wrong-owner, deny with `401`;
- otherwise lock the user, recheck `C == E`, insert the replacement row at `E`, delete `R`, and commit;
- mint the access token at `E` and return it only after the rotation commits;
- a replay of `R` after successful rotation is `401`.

Concurrent reset and rotation MUST linearize on the user lock:

- reset first: rotation fails; no renewal token is returned;
- rotation first: rotation may return epoch `E`; reset then purges the replacement row and advances to `E+1`; the returned access token is denied at the next admission even if its HTTP response arrives after reset;
- no interleaving may return a successful post-reset renewal carrying a usable prior epoch.

Identity assurance for the platform principal MUST complete before the session commit. No fallible post-commit step may be required before returning the token pair.

### 5.3 OAuth compatibility boundary

The observable complete-user OAuth access token MUST carry the same epoch and pass through the same session-writer fence as password login. OAuth registration completion uses the registration producer fence. If the separate provider Signup transaction creates the account and its first refresh row together, that transaction MUST persist the initial epoch and issue the first access token at that epoch.

The independent OAuth intent lot retains ownership of transaction consumption, intent-specific outcomes, cookies, PKCE, callback handling, and link/relink behavior. This contract requires only that any intent that actually issues a platform session use the shared epoch-aware producer. An intent that must not issue a session MUST NOT acquire one as a side effect.

The auxiliary JWT currently generated and discarded inside `OAuthService::process_callback` is not an observable session. It MUST NOT become an alternate issuance path; the OAuth lot MAY remove or align it without changing intent semantics.

## 6. Recovery transaction and observable outcomes

### 6.1 Request and validation

`POST /iam/api/auth/password/reset-request`:

- returns the existing generic acknowledgement for known and unknown addresses;
- MAY create a reset token and publish the existing request event for an eligible account;
- MUST NOT change the password, epoch, refresh rows, account lock state, or access-token liveness;
- MUST NOT end or reopen a session.

`POST /iam/api/auth/password/reset-validate` is read-only and MUST NOT change those states.

Reset-email throttling is STORY-003.01 and is not defined here.

### 6.2 Successful confirmation

`POST /iam/api/auth/password/reset-confirm` MUST perform one recovery transaction:

1. lock the user row;
2. compare the expected password snapshot;
3. revalidate reset-token owner, hash, expiry, and unused state;
4. set the new password hash;
5. increment `auth_epoch` exactly once with checked overflow;
6. delete all refresh rows for the account;
7. delete all reset-token rows for the account;
8. commit.

The success boundary is commit. The existing `200` confirmation message is returned only after commit and MUST contain no `access_token`, `refresh_token`, `user`, or `expires_in` field. A reset confirmation MUST NOT create a session.

### 6.3 Failure and retry

- Invalid, expired, used, wrong-owner, or stale reset proof: non-success response; password, epoch, refresh rows, reset tokens, and access liveness remain unchanged.
- Any database, overflow, purge, or commit failure: full rollback; no partial password change or partial revocation.
- Response lost after commit: the reset remains successful. Retrying the same link fails, the new password works, and prior access remains revoked.
- Old-password sign-in after commit: `401`.
- New-password sign-in after commit: a new explicit login may create epoch `E+1`; reset confirmation itself does not.

### 6.4 Authenticated password change exclusion

The signed-in current-password change MUST retain its current behavior:

- verify the current password;
- update the password and purge refresh/reset rows as today;
- do not increment `auth_epoch` and do not add immediate access-JWT revocation.

Tests MUST guard this exclusion so a shared writer refactor does not silently expand STORY-003 E3 to a different journey.

## 7. Error and availability matrix

- Revoked platform token: required route `401`; mesh denies with no principal headers.
- Malformed platform epoch: invalid token, required route `401`; mesh denies.
- Liveness endpoint/store timeout or failure: required route `503`; mesh denies; no fallback to cached signature validity.
- Non-platform recognized issuer: `not_applicable`; existing AuthN/AuthZ behavior continues, without a platform epoch lookup.
- Unknown platform account after a valid signed token: `revoked`; required route `401`.
- JWKS failure: existing ADR-0304/0308 fail-closed freshness rules remain in force.
- OpenFGA failure: existing authorization failure behavior remains in force; it is not converted into a session-revocation signal.
- Signing-provider failure: no new login/refresh token; already-issued tokens still require liveness.

No error response or log may contain the raw access token, refresh token, reset token, password, credential, or full signing reference.

## 8. Integration oracles

These are the required integration oracles. Section 11 records which have been executed; every unexecuted or partial oracle remains a delivery gate.

### OR-01 — Reset request has no session effect

Arrange a real password login with access and refresh tokens at epoch zero. Request a reset for the known address. Assert:

- generic acknowledgement matches the unknown-address acknowledgement;
- epoch is unchanged;
- the old access token still reaches a protected direct route;
- the old refresh token still rotates;
- old-password login still succeeds;
- no account lock or session purge occurred.

### OR-02 — Successful reset is a commit, not a request

Arrange two devices/sessions. Confirm a valid link. Assert:

- `200` with the existing message and no session fields;
- new password works only through a separate explicit login;
- old password fails;
- epoch increases by exactly one;
- all prior refresh rows and reset-token rows are absent;
- no session is created by confirmation itself.

### OR-03 — Prior direct access is denied immediately

Capture an unexpired pre-reset access token. After reset commit, without waiting for its 15-minute expiry, call a protected IAM route and representative protected Hive, Manifesto, and Telegraph routes in direct mode. Assert `401` and no command/effect. A separate user's token remains usable.

### OR-04 — Prior mesh access is denied immediately

Send the same pre-reset token through the opt-in mesh path. Assert `ext-authz` denies and creates no `x-principal-*` headers. Repeat with a forged client principal header and with a non-gateway mTLS peer; both remain denied.

### OR-05 — Renewal cannot reopen a reset session

After reset commit, submit every pre-reset refresh token. Assert `401`, no replacement row, and no access token. A replay of any token returned by a rotation that committed before reset is also `401`.

### OR-06 — Concurrent renewal and reset

Use the real PostgreSQL user-row barrier to force both orderings:

- reset first: renewal is `401`, no token pair;
- rotation first: renewal may be `200`, but after reset commit its access token is `401` and its replacement refresh row is gone;
- no run returns a post-reset `200` renewal whose token is usable.

Assert the final password, epoch, refresh count, and reset-token count, not only HTTP status.

### OR-07 — Concurrent and stale resets

Race two valid reset links and a stale credential snapshot. Assert exactly one winner, one epoch increment, the winner's password, no surviving refresh/reset rows, and invalid-token denial for losers. A stale proof obtained before the winner MUST NOT issue a session afterward.

### OR-08 — Rollback atomicity

Inject a fixture-owned failure into epoch update or refresh purge. Assert confirmation fails and password, epoch, refresh rows, reset tokens, and old access liveness are unchanged. A subsequent old-refresh rotation proves the rollback.

### OR-09 — Legacy access and refresh compatibility

Use a real pre-feature token without `auth_epoch` and a refresh row defaulted to epoch zero. Before the first reset, both remain usable. After the first reset, the legacy access token is `401` and the legacy refresh token is `401`. A new epoch-one login works immediately, including in the same wall-clock second as reset.

### OR-10 — Every producer carries the current epoch

Exercise password login, verified registration completion, complete-user OAuth login, and refresh. Decode each real access JWT and assert the canonical epoch claim and a persisted refresh row at the same epoch. After reset, every previously captured token is denied; each new explicit sign-in carries the incremented epoch.

### OR-11 — Liveness failure is fail-closed

Make the authoritative liveness read unavailable while JWKS and the access JWT remain valid. Assert required direct routes return `503` with no effect, mesh denies with no principal headers, and no cache or retry produces an allow. Restore the store and assert the same token is denied after reset.

### OR-12 — Issuer and account isolation

Use the same `sub` under a recognized organization issuer and under the platform issuer. The organization principal follows its existing rules and is not assigned the platform epoch. A platform reset does not revoke the organization identity, alter OpenFGA tuples, or affect another platform user. No suffix-based issuer alias is accepted.

### OR-13 — Request, validate, failed confirm, and successful confirm are distinguishable

For the same account, assert the full state tuple after each step:

- request: password/epoch/refresh/access unchanged;
- validate: unchanged;
- failed confirm: unchanged;
- successful confirm: new password, epoch advanced, refresh/reset rows removed, prior access denied, no new session.

### OR-14 — Authenticated change remains unchanged

Exercise the signed-in current-password change. Assert its current refresh purge behavior and unchanged epoch. Assert that this test does not acquire the new immediate access-revocation oracle reserved for recovery.

### OR-15 — Boundary inventory has no bypass

Enumerate every `.authenticated()` and optional-auth route in IAM, Hive, Manifesto, Telegraph, the monolith, and mesh manifests. For each protected family, prove both a positive pre-reset admission and a negative post-reset admission. A bespoke JWT decoder, direct service port, nested router, or optional-auth elevation path MUST NOT bypass the shared decision.

### OR-16 — OAuth intent compatibility

Prove that linked Login and provider Signup confirmation use the epoch-aware issuance boundary. For Signup, assert that the new account, first refresh row, and returned access token agree on their initial epoch. Verify that Link and Relink create no session. Preserve the OAuth transaction and browser oracles; do not replace them with session tests.

## 9. Rollout and rollback gates

1. Add the epoch fields additively with epoch-zero defaults; inspect real persisted state before applying. The IAM migration `m20261124_000001_auth_epoch` implements the additive columns and canonical-value constraint, and refuses to drop an advanced epoch.
2. Deploy epoch-aware issuance and the liveness endpoint.
3. Deploy direct admission to all known consumers and the monolith.
4. Deploy `ext-authz` liveness for opt-in mesh.
5. Verify OR-01 through OR-16 at the integrated revision, including failure injection and both reset/renewal orderings.
6. Enable enforcement only when the boundary inventory is complete. Until then, report the E3 guarantee as not delivered.
7. Rollback MUST NOT decrement epochs or reissue pre-reset tokens. Disabling enforcement after a reset is a security regression requiring explicit approval.

The implemented migration identifier is `m20261124_000001_auth_epoch`. The
shared SDK revision, public endpoint and feature configuration are recorded in
the evidence status; no deployment topology is reserved by this contract.

## 10. Approvals and unresolved choices

The product owner accepted the synchronous fail-closed availability cost, epoch-zero legacy rule, admission-time revocation without in-flight cancellation, recovery revocation of OAuth-issued platform sessions, and unchanged signed-in password-change behavior.

Before enforcement, the authoritative-store topology, timeout/capacity budget,
complete route inventory and trusted workload path remain unresolved. The
shared SDK/public seam is implemented and published. ADR acceptance does not
authorize production access; the later SDK/gitlink authorization remains
limited to implementation and publication.

## 11. Evidence status

The contract is grounded in the published ADR and static inspection of the pinned baseline. Native evidence on the IAM implementation base includes:

- a pre-implementation direct stale-access red (`200` instead of `401`);
- 89/89 `iam-domain` and 155/155 `iam-infra` unit tests;
- 27 passing HTTP/PostgreSQL reset tests, exactly the known stale-access red and one opt-in ignore;
- 8/8 migration tests;
- `session_writer_concurrency` passed 2/2 in campaign
  `it-e3f21472774445408e02e027f22a33cd`, with one fixture created/removed and
  zero campaign-owned running;
- complete direct Clippy production/tests lanes passed with valid JSONL reports
  and no finding attributable to an added line after the test-only correction;
- zero campaign-owned running fixtures after cleanup.

The published SDK revision is `42084b803b6a926f482a4030a0ed127447b21072`.
Its local qualification passed 171/171 unit tests, direct HTTP 6/6, mesh 4/4,
HTTP client 4/4, both Clippy lanes and 459/459 new executable lines; Linux
Actions and Sonar passed with 99.7% new coverage. The IAM endpoint passed 6/6,
`ext-authz` passed 14/14 lib tests plus 11/11 Mesh standin and 2/2
publisher/outage tests, and `iam-domain` 89/89 and `iam-infra` 155/155 passed
again. The filtered Mesh liveness oracle passed after final test cleanup.
Native HTTP/PostgreSQL `auth_password_reset` passed 28/28 with one opt-in
ignore, including the direct stale-access `401` oracle; one owned fixture was
created/removed, with 15 containers before/after and zero owned running.
Native `token` passed 22/22, including same-refresh concurrency with one winner
and two unauthorized losers; one owned fixture was created/removed with the
same 15-container before/after inventory. Native `session_writer_concurrency`
passed 2/2 at the integrated revision, covering one reset winner with no
surviving old session and purge-failure rollback; its one owned fixture was
removed with zero owned running. Native `internal_provider_token` passed 13/13,
`revoke_provider_token` 10/10 and `user` 26/26 after aligning absent-account
oracles with admission-time denial; each campaign removed its one owned
fixture with 15 containers before/after and zero owned running. Final direct
Clippy production/tests lanes passed with valid JSONL reports and zero findings
attributable to added lines.

The runtime-relevant sources exercised by the HTTP/PostgreSQL and migration campaigns are byte-identical to the later corrected envelope. The shared SDK and IAM/`ext-authz` wiring are implemented; direct authority failure, optional-auth, legacy admission, real deployed mesh, trusted workload path, boundary inventory and deterministic late-response rotation oracles remain pending. This contract is accepted as the target behavior, not as evidence that STORY-003.03 is complete.
