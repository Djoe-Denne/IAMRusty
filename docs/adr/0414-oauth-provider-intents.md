# ADR-0414 — Separate provider Login, Signup and explicit Link intents

- **Status:** Proposed
- **Reality:** Unimplemented
- **Date:** 2026-11-24
- **Decision owner:** User, through the coordinator; this draft is not ratified.
- **Scope:** STORY-007.01 at task revision `6912d9892b0845f2527c0ac9de088a7d82cfc11c`.
- **Pinned source baseline:** `b2362c6b40cd562b36c4b9800ac4fa4d53d1ac3f` (`origin/master` at admission).
- **Related:** STORY-007, STORY-006, STORY-005; ADR-0305, ADR-0409, ADR-0412, ADR-0413; [observable contract](../contracts/oauth-provider-intents.md); [ADR 0316](0316-password-recovery-session-liveness.md) for the shared session boundary.

## Context

The current public provider entry is `GET /iam/api/auth/{provider}/login`. Its
`OAuthOperation::Login` transaction reaches `OAuthService::process_callback` and
`find_or_create_user`, which combines five different decisions:

1. require a provider email;
2. resolve `(provider, profile.id)`;
3. otherwise attach an existing account by email when the provider marks that
   address verified;
4. otherwise persist an incomplete user and primary email; and
5. issue a session for a complete user, or a registration capability for an
   incomplete user.

This conflicts with the approved separation of provider sign-in, signup and
explicit linking. In particular, a provider email is not authority to enter an
existing account, and an already linked identity must not lose access merely
because its current provider profile omits or changes email. Persisting an
account before signup confirmation also leaves an abandoned account draft and
provider link. Finally, signup completion issues a first session only when the
supplied address was already marked verified, although STORY-006 E1 approves
first issuance after provider authentication and signup completion without a
separate AIForAll email-verification step.

ADR-0412 already supplies the required browser security machinery: a durable,
browser-bound, single-consumption transaction with provider, intent, target,
redirect, expiry, PKCE and an independent authentication nonce. ADR-0413 keeps
OAuthConnect outside IAM as a vendor-neutral authenticator returning a minimal
normalized profile. This proposal changes **IAM's interpretation of that
profile and the intent admitted at START**, not either trust boundary.

Security takes precedence over response compatibility and migration effort.
An email match, a client-supplied operation, a callback query, or an old
registration token must never become authority for a link or a session.

## Decision

### 1. Three intent-specific application paths, one provider authenticator

Introduce distinct, closed application intents:

- **ProviderLogin** — public; resolves an existing linked account only;
- **ProviderSignup** — public; prepares or completes exactly one new account;
- **ProviderLink** — authenticated and explicit; adds a method to the account
  authenticated at START.

Keep **Relink** as the existing, separate authenticated operation under ADR-0412
and ADR-0413. This proposal neither merges it into Link nor changes its email,
identity-collision, token-replacement or target rules.

Login, Signup and Link use the same `FederatedOAuthClient`/OAuthConnect v2
completion and the same browser transaction writer. Relink retains its existing
provider completion and writer path under ADR-0412/0413; this proposal does not
rewrite its distinct business rules. OAuthConnect remains stateless with respect
to accounts, linking, browser binding and sessions. No new provider profile,
account service, nonce cache or dependency is introduced.

The transaction operation is closed and persisted, not inferred from a request
parameter at callback. Conceptually the admitted set becomes `Login`, `Signup`,
`Link { user_id }`, `Relink { user_id }`. The exact Rust enum, command and route
names are implementation choices within the observable contract.

### 2. Identity authority is the provider instance plus opaque subject

The external identity key is:

```text
(configured provider instance / canonical provider slug, provider profile.id)
```

The subject is opaque, non-empty and interpreted only within that configured
instance. It is not an email, username, platform UUID, organization identity or
a subject shared across providers. The same opaque string under two provider
instances identifies two different external identities. OIDC `iss` is pinned and
validated by OAuthConnect under ADR-0413; IAM does not flatten issuer and subject
into a global email or UUID lookup.

ProviderLogin resolves this key first and only. A different or absent email is
irrelevant to identity resolution. Login never retargets an existing link,
imports an address, changes a primary/secondary flag, or creates an account.
The account response uses its saved primary address; if none exists, email is
absent/null rather than copied from the provider. No lookup by email or username
is permitted in ProviderLogin.

The existing `provider_tokens(provider, provider_user_id)` unique key remains the
compatibility authority for stored links. This ADR does not migrate links,
replace provider tokens with a new identity table, or prove GitLab OIDC subject
continuity; ADR-0413's external cutover gate remains.

### 3. Login is read-only with respect to business accounts and links

After a valid provider completion, ProviderLogin has exactly two outcomes:

| Resolved state | Outcome |
|---|---|
| Exact linked identity belongs to a complete account | Open a session for that account after applicable authentication checks. |
| No exact link | Refuse without creating an account/link or issuing a session. |
| Exact link belongs to an incomplete account | Refuse without completing/retargeting it or issuing a session; existing draft disposition is OQ-3. |

An unlinked identity is refused even if its email is registered, verified, equal
to the signed-in browser's current account, or otherwise attractive. There is no
implicit link, account merge, draft creation, provider-token refresh/upsert, or
address import on this path. A conflicting/incomplete stored link is a storage
integrity failure, not permission to fall back to another account.

Cancellation, provider refusal, invalid browser bindings and provider failure
leave business state unchanged. The OAuth transaction itself follows ADR-0412:
a valid callback consumes the transaction before external completion; a later
failure requires a new START and does not roll back that protocol consumption.

### 4. Signup persists a complete account atomically at confirmation

ProviderSignup admits only an unlinked provider identity. It does not select an
existing account by email. Its pre-confirmation state is a short-lived signup
preparation capability bound to the consumed browser transaction and provider
identity. It is **not** an account, an account draft, a provider link or an
authentication/registration bearer.

After valid provider completion:

| Provider state | Pre-confirmation outcome |
|---|---|
| Identity already linked | Offer the existing provider sign-in outcome; create nothing. |
| Supplied address is already registered | Return non-identifying existing-account guidance; create no account, draft, link or resumption email. |
| Supplied address is available | Prepare signup; the person confirms a valid username. |
| No supplied email | Outside this lot. Do not invent collection or account creation. |

“Registered” uses the repository's persisted address identity and uniqueness
constraint, regardless of verification status. Address canonicalization is not
changed by this ADR. A concurrent registration that wins after the availability
check is resolved by the database constraint and mapped to the same
non-identifying existing-account outcome; a second account must not be created.

At successful confirmation, one primary-writer transaction must:

1. revalidate the signup preparation, provider scope, availability and expiry;
2. recheck provider-link and address uniqueness under write constraints;
3. create exactly one complete user with the confirmed username;
4. create exactly one primary address and exactly one provider link/token row;
5. persist the first refresh session; and
6. commit before returning access and refresh credentials.

The supplied primary address is verified **if and only if** the normalized
provider profile contains the exact same address and an affirmative
`email_verified` assertion. False, absent, malformed or non-exact proof leaves
it unverified. Provider authentication still satisfies this signup's admission;
address verification remains a separate stored property. No separate AIForAll
email confirmation is required before the first session for this approved
supplied-email flow.

If any write or session persistence fails, no partial account, address, link or
session may be committed. Access minted before a failed final commit must never
be returned. A replay of the signup preparation creates no second account or
session. Whether an uncertain commit can be safely retried is a storage-idempotency
question for implementation, not permission to duplicate effects.

The current shared `complete-registration` handler is also used by email/password
signup. Provider signup must not globally remove the verified-email session gate
from that other flow. Isolate the provider completion semantics, or version the
completion boundary explicitly, while preserving the email/password behavior
recorded in `test_complete_email_first_flow`.

### 5. Explicit Link keeps its own authority and mutation rules

Link START remains authenticated, binds the verified platform account into the
transaction, and requires provider confirmation. Its callback remains authorized
by the persisted target plus browser binding under ADR-0412, not by a Bearer
header on the callback.

For the preserved STORY-005 E1–E3 behavior:

- success adds the provider method to the same account without removing existing
  methods;
- a different provider email may be imported as a **secondary** address without
  changing the primary address; and
- cancellation/provider failure adds no method and changes no existing method.

The existing Link conflict checks remain security requirements: an identity
already linked elsewhere and an address belonging to another account both refuse
the entire operation with no reassignment, transfer or partial address import.
An available imported secondary address is verified only on exact affirmative
provider proof. Refusal reasons must not identify the other account.

Link and Relink are not silently changed by this proposal. Their current
sequential email/link/token writes are not proof of all-or-nothing refusal under
every storage failure; the contract requires no business mutation on the
approved refusal/cancellation cases and requires the final writer design to
preserve that boundary. Recency window, verified-account prerequisite and exact
UX remain outside the reconciled E1–E3 scope.

### 6. Public entry points and callback dispatch

Add a distinct public provider signup entry point while retaining the existing
public login entry point and callback URLs. Use this conceptual mapping:

| Intent | START | Callback |
|---|---|---|
| Login | `GET /iam/api/auth/{provider}/login` | `GET /iam/api/auth/{provider}/callback` |
| Signup | `GET /iam/api/auth/{provider}/signup` (proposed path) | `GET /iam/api/auth/{provider}/signup-callback` (proposed path) |
| Link | authenticated `GET /iam/api/auth/{provider}/link` | existing shared `.../callback` |
| Relink | authenticated `GET /iam/api/auth/{provider}/relink-start` | existing `.../relink-callback` |

A callback accepts only the operation admitted by its exact router. Login,
Signup, Link and Relink capabilities cannot be substituted across routes. The
existing shared login/link callback may be retained with an exact
`Login | Link` route-intent guard; a single permissive dispatcher is not
required. The Signup path and its final wire names require review before
implementation because they are new public surface.

The browser nonce cookie remains host-only, HttpOnly, SameSite-appropriate and
Secure outside the explicit local exception. It grants no account authority.
PKCE, nonce, state, redirect, expiry, target and writer-consumption checks are
unchanged. Callback error responses still consume only an otherwise valid
matching transaction; they do not bypass validation.

### 7. Compatibility and cutover

The following are intentional behavior changes, not regressions to preserve:

- verified-email association during ordinary Login is removed;
- email-less linked Login succeeds;
- an unlinked Login no longer prepares signup;
- abandoned provider Signup no longer leaves an account draft/link; and
- eligible supplied-email Signup issues its first session even when the address
  is stored unverified.

Preserve provider slugs, callback URLs, UUID accounts, existing link keys,
access/refresh token authority and OAuthConnect v2 DTOs. Preserve email/password
signup/login and existing Link/Relink behavior outside the stated corrections.
There is no bulk relinking or account migration.

Cutover order:

1. ratify this ADR and the contract, resolve the public-route/error decisions,
   and agree the treatment of existing incomplete provider drafts;
2. land protocol/domain/writer compatibility before switching public behavior;
3. deploy the new IAM consumer without changing OAuthConnect;
4. switch Login and Signup entry points together so neither retains the unsafe
   combined behavior;
5. verify native integration oracles, then the affected browser/provider journey
   on an authorized runtime; and
6. remove superseded combined-path code only after its consumers and tests have
   an explicit disposition.

Pending transactions may finish only under their originally persisted intent
and security bindings; they must never be reinterpreted as another intent. An
outstanding combined-flow `registration_required` capability must not authorize
an unapproved account/link/session transition under the new contract; its
existing-draft disposition is OQ-3. Rollback must not restore verified-email Login
association. If safe rollback
cannot preserve the approved security boundary, fail closed and require a new
START rather than reintroducing the vulnerable behavior.

## Alternatives considered

### A. Keep one combined Login/Signup transaction and branch after profile lookup

**Rejected.** It preserves the current route but makes account selection depend
on a profile email at the same boundary that must forbid email association. It
also conflates abandoned signup with an account draft. A discriminator added
only after lookup would not establish intent authority and would retain the
security-critical ambiguity.

A single transaction **mechanism** is still appropriate; the rejected option is a
single ambiguous business intent, not shared writer infrastructure.

### B. Separate intent-specific public entry points and persisted transactions

**Selected.** It makes START authority explicit, permits route-specific callback
guards, keeps Link/Relink targets authenticated, and allows Login to be strictly
read-only while Signup owns all creation. Cost: a new public entry/callback,
consumer migration and coordinated cutover. These costs are preferable to
retaining email-based account selection.

### C. Keep one browser START, then ask for intent after provider authentication

**Not selected.** It avoids a second provider authorization but lets an
unauthenticated callback choose between account-affecting outcomes. To be safe
it would need another durable, browser-bound intent-confirmation step and still
could not authorize Link without platform authentication. It adds UX and state
without removing the need for distinct application operations.

### D. Infer intent from an email match or the presence of a username

**Rejected as unsafe.** Email is not identity authority, and incomplete accounts
are legacy data rather than evidence of user intent.

### E. Move account/link/session decisions into OAuthConnect

**Rejected.** It contradicts ADR-0409/0412/0413, creates a second account authority
and expands the connector's trust boundary. OAuthConnect authenticates the
external identity; IAM owns accounts, links and sessions.

## Consequences

### Positive

- Ordinary Login cannot take over an account by email.
- Linked Login is independent of current provider email.
- Signup has one explicit, atomic creation/session boundary.
- Link retains the strong existing browser and target bindings.
- Security invariants become directly testable at intent-specific boundaries.

### Costs and risks

- New public route/response vocabulary and client migration.
- A provider-specific signup completion writer and preparation capability are
  needed; the current shared registration service cannot simply be relaxed.
- Existing combined-flow tests and k6 scenarios encode old behavior and require
  explicit migration, not silent weakening.
- Existing incomplete OAuth users and outstanding registration capabilities
  need a reviewed disposition before cutover.
- The first access token is minted before its refresh row is persisted in the
  current architecture; the final commit must still precede credential release.
- Security fixes may strand in-flight attempts. A fresh START is the safe
  recovery, not an email fallback or transaction replay.

## Shared seam with STORY-003.02 — not decided here

Provider Login, provider Signup completion, email/password login and registration
completion all produce access/refresh credentials. The existing shared writer is
`AuthenticationSessionWriter`, implemented by
`SeaOrmAuthenticationSessionWriter`:

- `issue` locks the user and compares the expected password hash before storing
  a hashed refresh token;
- `complete_registration` updates the username and optional refresh session in
  one user-row transaction;
- password login obtains a password-factor proof and delegates session issuance;
- OAuth currently calls `issue` with the account's password hash;
- refresh rotates through `RefreshTokenWriteRepository::rotate`, which locks the
  user and atomically replaces the old refresh row.

Assumptions exposed to STORY-003.02:

1. All initial sign-in producers must persist a usable refresh session before
   returning credentials, and must not weaken the writer's stale-proof checks.
2. OAuth-only accounts have `password_hash = None`; the shared writer currently
   compares that snapshot, but that is not evidence of a complete liveness or
   reset-revocation policy.
3. Access JWT issuance and refresh persistence are separate operations.
4. Renewal uses a different writer path from initial issuance.

This ADR does **not** select reset/liveness enforcement, access-token deny lists,
session versions, credential-generation fencing or renewal-race policy. Those
belong to STORY-003.02. The coordinator must reconcile the shared session contract
before dependent implementation or tests are dispatched.

## Open product/architecture decisions requiring approval

1. **New public Signup route and callback names.** `/signup` and
   `/signup-callback` are proposed, not approved product copy or routing.
2. **Existing-account guidance and refusal vocabulary.** This draft defines
   non-identification and no-mutation invariants but does not ratify final HTTP
   statuses, error codes, localization or screens.
3. **Pre-existing incomplete OAuth accounts/drafts and outstanding combined-flow
   registration capabilities.** Keep, expire, repair, remove or provide a narrow
   compatibility path is not decided. No deletion, bulk migration or implicit
   new-contract completion is authorized.
4. **Preparation storage and replay policy.** A durable one-time capability is
   recommended; its schema, retention and uncertain-commit idempotency require
   implementation design. A self-contained token is acceptable only with the
   same one-time security and atomic effects.
5. **Rollback/failure recovery UX.** Fresh START is the safe technical fallback;
   the interface and operational procedure are not specified.

Deferred MFA, navigation destinations, provider outage recovery, missing-email
signup, username prefilling, exact copy, account merging and provider-resource
access are not promoted into requirements by this draft.

## Validation status

This proposal is grounded in static inspection of the pinned baseline.
The [OAuth callback suite](../../services/IAMRusty/tests/auth_oauth_callback.rs)
and [browser transaction suite](../../services/IAMRusty/tests/oauth_browser_transactions.rs)
are evidence candidates, not proof that the proposed intent separation is implemented.
