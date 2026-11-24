# Provider intent observable contract — Proposed companion to ADR-0414

- **Status:** Proposed; not an implementation authorization.
- **Revision:** STORY-007.01 task `6912d9892b0845f2527c0ac9de088a7d82cfc11c`.
- **Source baseline:** `b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f`.
- **ADR:** [ADR 0414](../adr/0414-oauth-provider-intents.md).
- **Shared session boundary:** [Password recovery session liveness](password-recovery-session-liveness.md) is a separate proposal.

This contract defines observable behavior and mutation boundaries. Rust type
names, SQL shape, exact new route names and final public error codes remain
implementation/review details. Security invariants are normative; the current
wire shapes are compatibility references, not permission to preserve unsafe
behavior.

## 1. Actors and authorities

| Actor/input | Authority | Not authority |
|---|---|---|
| OAuthConnect v2 `ProviderAuthentication` | External authentication plus normalized `{id, username, email?, email_verified}` for the configured provider instance | AIForAll account selection, linking permission, session |
| Persisted provider transaction | Admitted intent, provider, redirect, expiry, browser binding, optional authenticated target, PKCE/nonce context | A new intent supplied at callback |
| Browser nonce cookie | Proof that the callback presenter is the browser that received START | Account authentication or link permission |
| Platform Bearer at Link/Relink START | Authenticated account bound into the transaction | Callback authorization by itself |
| Provider profile email | Profile data and, for Signup/Link, an address subject to availability/proof rules | Identity lookup or account association |
| `(provider instance, profile.id)` | External identity key | Email/username equivalence |
| Signup preparation capability | One pending Signup attempt after valid provider completion | Account draft, provider link, access or registration bearer |

No query parameter, path alias, unsigned state, provider token, cookie or signed
registration capability can manufacture a different operation or target.

## 2. Common protocol rules

These rules apply to Login, Signup, Link and Relink:

1. Resolve the configured canonical provider and exact registered redirect.
2. Persist the BEGIN transaction before returning a successful START or calling
   OAuthConnect authorize.
3. Require HMAC state, browser cookie, provider, admitted operation, target,
   redirect, expiry and PKCE/nonce context before consuming the transaction.
4. Consume once on the authoritative writer and commit before code completion.
   Invalid bindings do not consume; an expired row is not made consumable.
5. Complete authorization exactly once through OAuthConnect v2. No v1 fallback,
   second profile request or retry on the consumed capability.
6. A valid vendor-error callback terminates the same validated transaction
   without code exchange. A later failure is not replayed; a new START is needed.
7. Never put state, code, verifier, authentication nonce, provider tokens,
   access/refresh tokens or preparation secrets in errors or logs.
8. Never issue credentials merely because the provider transaction was consumed.

Transaction consumption is protocol state, not a business account mutation.
“No business mutation” below permits only ADR-0412 transaction lifecycle changes.

## 3. Intent-specific START and callback surface

| Intent | Admission | Proposed public surface | Allowed persisted operations |
|---|---|---|---|
| Login | Public | Existing `GET /iam/api/auth/{provider}/login`; existing `GET .../callback` | `Login` |
| Signup | Public | Proposed `GET /iam/api/auth/{provider}/signup`; proposed `GET .../signup-callback` | `Signup` |
| Link | Authenticated platform account | Existing `GET /iam/api/auth/{provider}/link`; existing shared callback | `Link { user_id }` |
| Relink | Authenticated platform account | Existing `GET .../relink-start`; existing `GET .../relink-callback` | `Relink { user_id }` |

The existing shared callback may accept exactly `Login | Link`; it must reject
`Signup` and `Relink`. The Signup callback accepts only `Signup`. The Relink
callback accepts only `Relink`. A valid capability for another route/intention is
an invalid-state refusal, with no business effect.

Link/Relink START failures refuse before creating a transaction or setting a
success cookie. Link/Relink callbacks continue to use state + browser binding
under ADR-0412; no new Bearer requirement is added to callbacks.

The Signup route names are **open decision OQ-1**. Until approved, treat them as
logical entry points rather than instructions to expose new URLs.

## 4. ProviderLogin

### 4.1 Preconditions

- Consumed operation is exactly `Login`.
- Provider/redirect match the consumed capability.
- OAuthConnect returned a complete, valid normalized profile.
- `profile.id` is non-empty and scoped to the configured provider instance.

Email is not a precondition.

### 4.2 Resolution and outcomes

Lookup only `(provider instance, profile.id)`.

| Stored state | Observable outcome | Business mutation | Credentials |
|---|---|---|---|
| Link resolves to one complete account | `login` success for that account's UUID and saved primary profile | Persist one new session; no change to users, addresses, provider links/tokens or existing refresh rows | Access + refresh after session persistence |
| No link | Non-identifying provider-login refusal/guidance; no signup fallback | None | None |
| Link resolves to an incomplete account | Non-identifying refusal; do not complete or retarget it | None | None |
| Link is ambiguous/corrupt | Fail closed as storage/integrity failure | None | None |
| Provider email differs, is unverified, or is absent | Same result as the exact linked state above | No address/link change | As resolved above |
| Provider completion/cancellation fails | Generic authentication failure | None | None |

`user.email` in a Login response is the saved primary address, or absent/null if
there is no primary row. It is never the current provider email. Existing
`operation: "login"`, `user`, `access_token`, `expires_in`, `refresh_token`
fields can remain for successful linked Login.

### 4.3 Forbidden behavior

- No `find_by_email` or username fallback.
- No implicit Link, account merge or draft.
- No provider-token refresh/upsert on Login.
- No primary/secondary address creation or verification change.
- No session for an unlinked identity, even with a verified matching email.
- No session from a registration/preparation capability.

## 5. ProviderSignup before confirmation

### 5.1 Preconditions

- Consumed operation is exactly `Signup`.
- Valid OAuthConnect completion and non-empty provider-scoped subject.
- No existing provider link for that exact identity.

### 5.2 Decision table

| Provider state | Observable outcome | Allowed mutation |
|---|---|---|
| Exact identity already linked | Offer existing-provider sign-in guidance; do not create | None |
| Supplied email matches a registered address, verified or not | Non-identifying existing-account guidance | No account, draft, link, address change or resumption email |
| Supplied email is available | Return a Signup preparation capability and editable username-confirmation data | Only one-time preparation state; no `users`, `user_emails`, `provider_tokens` or `refresh_tokens` row |
| Email absent | Unsupported/refused in this lot | None |
| Provider completion/cancellation fails | Generic failure | None |

Existing-account guidance must not reveal the account UUID, username, whether a
password exists, verification state, or another person's provider links. It must
not send a resumption email. Exact screen/copy/status is **OQ-2**.

A preparation capability:

- is bound to provider instance, opaque subject, supplied exact email and its
  verification proof;
- is bound to the consumed Signup transaction/browser origin;
- is single-use and short-lived, no later than the signup attempt's approved
  retention window;
- has a dedicated type/audience and is rejected by account Bearer guards and by
  the email/password registration-completion endpoint;
- carries no access/refresh authority and creates no account UUID; and
- is redacted in Debug/logs/errors.

Its precise storage (durable writer row versus equivalently one-time signed
capability) is **OQ-4**. A bearer self-contained token without replay protection
is not sufficient.

### 5.3 Preparation failure

Provider success does not permit a partial account on preparation failure.
Return a retryable failure with no business rows or credentials. After a valid
transaction has been consumed, the person starts a new provider attempt; the
preparation cannot make the consumed callback replayable.

## 6. ProviderSignup confirmation

### 6.1 Inputs and revalidation

Confirmation accepts only the Signup preparation capability and a valid,
available username. It does not accept an account ID, provider subject, email,
provider token, target user or arbitrary operation from the caller.

On the authoritative writer, revalidate:

1. capability signature/format, dedicated audience, expiry and unused state;
2. exact provider instance and opaque subject;
3. absence of an existing provider link;
4. absence of the supplied address under the persisted address uniqueness rule;
5. username validity and uniqueness;
6. exact email-verification proof; and
7. the provider-authenticated Signup admission bound into the capability.

A concurrent provider link or address registration must lose through uniqueness
constraints, not create a second account or transfer a link. The caller receives
a non-identifying conflict and no credentials.

### 6.2 Atomic success effects

One commit must contain all of the following:

- exactly one complete `users` row with the confirmed username;
- exactly one primary `user_emails` row for the supplied address;
- exactly one `provider_tokens` link for `(provider, provider_user_id)`;
- exactly one valid, hashed `refresh_tokens` row for the first session; and
- consumption of the Signup preparation.

The primary address's `is_verified` value is:

```text
profile.email == supplied_email AND profile.email_verified == true
```

Only affirmative proof for that exact address sets `true`. False, omitted or
non-exact proof sets `false`. There is no separate AIForAll email-verification
step before the first session for this supplied-email provider flow.

### 6.3 Observable success

After commit, return HTTP 200 with the existing completion-compatible profile
and credential names:

```json
{
  "user": { "id": "<uuid>", "username": "<confirmed>", "email": "<supplied>" },
  "access_token": "<platform access JWT>",
  "expires_in": "<positive configured lifetime>",
  "refresh_token": "<persisted opaque refresh token>"
}
```

An unverified supplied address does not change this success contract. The access
JWT must use the configured platform issuer/audience and identify the newly
created UUID. The refresh row must contain the hash of the returned token.

### 6.4 Failure and replay

| Failure | Required result |
|---|---|
| Invalid/expired/used preparation | 400-class invalid-capability refusal; no credentials or business mutation |
| Username/address/provider-link conflict | Non-identifying conflict; no partial rows or credentials |
| Any DB write/session insert/commit failure | No partial account/address/link/session; no returned credentials |
| Access signing succeeds but final persistence fails | Access is not returned and no usable refresh session survives |
| Replay/parallel confirmation | At most one account and one first session; losers create nothing |

No account remains able to sign in or use product features before successful
confirmation. After confirmation, a replay must not create another session.

## 7. Explicit ProviderLink — preserved STORY-005 E1–E3 boundary

Link remains a separate authenticated operation. This contract does not relax
its recency/verification prerequisites or define their exact values.

| Situation | Observable outcome | Mutation |
|---|---|---|
| Recent platform authentication, provider confirms an available identity | `operation: "link"` success for the same UUID | Add provider method; preserve existing methods |
| Provider email differs and is importable | Import it as secondary | Primary unchanged; secondary proof follows exact-address rule |
| Provider email absent | Link may succeed | No address import; primary unchanged |
| Provider cancellation/failure | Generic refusal | No method/address/token change |
| Provider identity already linked to this account | Refuse as already linked | No replacement or transfer |
| Provider identity linked to another account | Safe identity-conflict refusal | Both accounts and links unchanged |
| Provider address belongs to another account | Safe address-conflict refusal; entire Link refused | Both accounts, addresses and links unchanged |
| Callback target/browser/provider mismatch | Invalid-state refusal before business I/O | None |

For successful secondary import:

```text
is_primary = false
is_verified = (provider email_verified is affirmative for the exact address)
```

Do not change an existing same-account address's flags during Link. Preserve
existing response fields `operation`, `message`, `user`, `emails`,
`new_email_added` and `new_email` where compatible.

Link refusal/cancellation must not partially import an address and then fail to
save the provider method. The final implementation must make the business unit
atomic or otherwise prove the same observable all-or-nothing boundary. This is
a required invariant, not evidence that the current sequential repository calls
already provide it.

Relink is unchanged by this contract. Do not copy Link's identity/email policy
into Relink or infer new Relink rules from Signup.

## 8. Cancellation and refusal mutation matrix

Business snapshot means users (including password hash and username), every
email field, provider links/tokens, refresh rows and any Signup preparation.
Compare before/after without exporting secrets.

| Event | Login | Signup | Link | Relink |
|---|---|---|---|---|
| Invalid/missing state, cookie, provider, intention, target, redirect or expiry | No business change | No business change | No business change | No business change |
| Valid transaction, provider returns `error` | Consume only; no business change | Consume only; no business change | Consume only; no business change | Consume only; no business change |
| OAuthConnect complete refuses/malformed/tokens-only | No business change | No business change | No business change | No business change |
| Signup abandoned before confirmation | No business account/link/session; preparation may expire | Same | Not applicable | Not applicable |
| Unlinked Login, including verified email match | Refuse; no business change | Not applicable | Not applicable | Not applicable |
| Signup duplicate address or linked identity | Not applicable | No account/draft/link/session/email | Not applicable | Not applicable |
| Link email/identity conflict | Not applicable | Not applicable | Entire Link refused; both accounts unchanged | Unchanged existing rules |
| Replay of any consumed callback/preparation | No additional external completion or business effect | No second account/session | No repeated Link effect | No repeated Relink effect |

## 9. Compatibility and cutover oracles

### 9.1 Preserved contracts

- Existing Login and Link callback URLs and provider slugs.
- Existing Relink entry/callback URLs and ADR-0412 callback-without-Bearer rule.
- Access token algorithm, type, configured issuer/audience and platform UUID
  subject; opaque refresh token stored hashed.
- Existing Link response field names and same-account UUID.
- OAuthConnect v2 request/response DTOs and no-v1-fallback rule.
- Email/password signup/login behavior, including no session before email
  verification in that separate flow.
- Existing account/link keys; no bulk relink or account merge.

### 9.2 Intentional breaking behavior

Tests/consumers asserting any of the following must be explicitly migrated:

- Login association by verified email;
- Login requiring provider email for an already linked identity;
- Login creating an incomplete account or registration token;
- an incomplete OAuth account/link surviving abandoned Signup;
- empty access/refresh strings after eligible supplied-email provider Signup;
- shared callback accepting an operation not admitted by its router.

### 9.3 Cutover gates

1. ADR/contract approval and OQ-1/OQ-2/OQ-3 disposition.
2. Writer/migration compatibility deployed before the new consumer.
3. Login and Signup security switch deployed together; no dual-read fallback to
   email association.
4. Native integration oracles in §10 pass on the retained content.
5. Affected browser/provider scenario passes on an authorized runtime, with
   correlation and no secret export.
6. Old combined-path code removed only after consumer/test disposition.

Rollback must not restore email-based Login. Outstanding transactions retain
their original intent; they are not reclassified. An old combined-flow
`registration_required` capability is not a ProviderSignup preparation and must
not complete a new account/link/session until OQ-3 defines a safe compatibility
path. Recovery after a failed consumed attempt is a new START.

## 10. Behavioral integration oracles

These are proposed future oracles. **None was executed in this lot.** Use the
repository's existing IAM HTTP + PostgreSQL + OAuthConnect mock/TLS fixtures and
native Windows test runner. Preserve negative prerequisites and assert actual
provider completion receipts; a missing fixture or earlier refusal is not a PASS
for a later oracle.

| ID | Approved example / risk | Arrangement and action | Required observations |
|---|---|---|---|
| O1 | STORY-007 E1; identity scope | Link provider A subject `same-id` to account A and provider B subject `same-id` to account B; Login each; repeat through a second IAM listener | Each Login returns its own UUID; two distinct links remain; one completion per callback; configured issuer/audience; persisted refresh |
| O2 | STORY-007 E1/E2; email independent | Linked complete account; provider returns absent, different, and differently verified emails in separate attempts | Login succeeds to the linked UUID each time; saved addresses/links/tokens unchanged except no Login token upsert; new session persisted |
| O3 | STORY-007.02 E2; unlinked verified email | Existing account with verified address; unlinked provider returns that exact verified email | No session, account, draft or link; no address change; no registration capability; no email sent |
| O4 | STORY-007 E3 | Provider `error`, invalid code, connector refusal and malformed/tokens-only completion for Login | No access/refresh/registration credential; business snapshot unchanged; valid transaction consumed/purged; replay performs no second completion |
| O5 | STORY-006 E4 | Signup START with an already linked identity, including changed/absent email | Existing-provider sign-in guidance; no duplicate account/link/session/address import |
| O6 | STORY-006 E3 | Signup with an available provider identity and an already registered address, both verified and unverified variants | Non-identifying existing-account guidance; zero account/draft/link changes; no resumption email; no session |
| O7 | STORY-006 E1 | Signup with available supplied email; confirm username; verify first credentials immediately | 202 preparation contains no account bearer; before confirmation zero users/emails/links/refresh; after one completion request, 200 with one complete account, one primary address, one link and one persisted first refresh; no second provider authorization |
| O8 | STORY-006 E7 | Signup with exact `email_verified=true`, `false`, and omitted proof | Stored primary verification is respectively true, false, false; all eligible flows receive first session at completion; no AIForAll email verification required |
| O9 | STORY-006 E1/E3 concurrency | Race two Signup confirmations for same provider identity, same address, or same username | At most one account/link/first session; losers receive safe conflict with no partial rows or credentials |
| O10 | STORY-006 E5 | Obtain preparation then abandon, expire it, submit invalid username, fail a write, and replay it | No account can sign in/use product features; no partial users/emails/links/refresh; no credential returned on failed commit; at most one successful account |
| O11 | STORY-005 E1/E3 | Authenticated Link with different importable email; existing password/second provider present | Same UUID; existing methods unchanged; imported address secondary; primary unchanged; exact verification flag respected |
| O12 | STORY-005 E2 | Link cancellation, provider failure and replay | No new method/address/token change; no credential; consumed transaction cannot repeat completion |
| O13 | Link security regression | Link address owned by another account; provider identity owned by another account; forged target/query/cross-route capability | Whole Link refused; both accounts/links/addresses unchanged; safe reason; no other-account identifier; zero business writes |
| O14 | ADR-0412 preservation | Wrong/missing/duplicate cookie, state, provider, intention, target, redirect, expiry; two tabs; two replicas; writer failure | Existing 0412 outcomes remain: invalid binding denied before completion, independent tab states, one consumer, fail-closed storage errors, no fallback |
| O15 | Registration capability boundary | Present Signup preparation and old registration tokens to `/api/me`, Link/Relink START, password completion and refresh | All rejected as account authority; dedicated audience/type preserved; no session |
| O16 | Session seam | Inspect linked-provider Login, provider Signup completion, password Login and email/password registration completion; refresh each issued session | Each success returns only after a usable hashed refresh row exists; refresh rotates once; stale/revoked behavior remains governed by STORY-003.02, not asserted as a new policy here |
| O17 | Compatibility/cutover | Run affected existing suites and the browser journey after consumer migration | No stale verified-email Login expectation remains; no old combined Signup behavior remains; exact routes/cardinality/authority updated without weakening security checks |

### Suggested existing suites to extend or migrate

- `services/IAMRusty/tests/oauth_browser_transactions.rs`
- `services/IAMRusty/tests/auth_oauth_callback.rs`
- `services/IAMRusty/tests/auth_oauth_start.rs`
- `services/IAMRusty/tests/auth_oauth_provider_failures.rs`
- `services/IAMRusty/tests/auth_complete_registration.rs`
- `services/IAMRusty/tests/auth_username_flow.rs`
- `services/IAMRusty/tests/registration_usage_boundary.rs`
- `services/IAMRusty/tests/relink_provider.rs`
- `services/IAMRusty/tests/session_writer_concurrency.rs`
- `ops/tests/k6/scenarios/auth/oauth-browser.js`
- `ops/tests/k6/scenarios/auth/provider-link-email.js`
- `ops/tests/k6/scenarios/auth/provider-link-conflicts.js`

The k6 scenarios are compatibility consumers, not substitutes for PostgreSQL
mutation or writer-failure oracles. A browser observation of a JWT is not proof
of its signature or protected-route admission; use the configured verifier and a
real protected route.

## 11. Shared session assumptions handed to STORY-003.02

The following are observations, not reset/liveness decisions:

- Initial producers: OAuth Login (`OAuthUseCaseImpl::oauth_login`), provider
  Signup completion (proposed), password Login (`AuthService::complete_login`),
  and registration completion (`RegistrationServiceImpl::complete_registration`).
- Shared port: `AuthenticationSessionWriter`; production implementation:
  `SeaOrmAuthenticationSessionWriter`.
- `issue` locks the user row and compares `expected_password_hash`; OAuth passes
  the account's current hash, including `None` for OAuth-only accounts.
- `complete_registration` changes username and optional refresh row in one
  transaction.
- Password Login's factor store ultimately uses the same writer; refresh renewal
  uses `RefreshTokenWriteRepository::rotate`, not `issue`.
- Access mint and refresh persistence are separate. This contract requires no
  credential release before persistence but does not redesign mint atomicity.
- Reset currently purges refresh rows; already issued access JWT behavior is
  outside this contract.

Before dependent work, STORY-003.02 and the coordinator must agree whether the
shared writer's credential snapshot, initial issuance and renewal paths have the
required liveness/reset semantics. This lot chooses none of those policies.

## 12. Explicit non-requirements

This contract does not add or decide:

- deferred MFA or provider MFA sufficiency;
- return destination, navigation or unavailable-destination behavior;
- missing-email Signup collection;
- username prefilling;
- exact UI copy, localization or screen layout;
- provider outage recovery UX;
- recency window values;
- account merging or provider-resource access;
- reset/liveness/access-token invalidation policy; or
- production/provider-real cutover qualification.

Those remain deferred, owned elsewhere, or require separate product approval.
