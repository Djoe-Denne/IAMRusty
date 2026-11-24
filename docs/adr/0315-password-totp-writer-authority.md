# ADR 0315: Writer authority for password TOTP

- Status: Proposed
- Scope: IAM STORY-004. Recovery, replacement, provider sign-in, security
  recommendations, and password-reset redesign are separate.
- Observable contract: [IAM password TOTP](../contracts/iam-password-totp.md).

## Decision

Use the existing password login endpoint for both submissions. A successful
password check for an enrolled account may produce a tokenless challenge, never
an intermediate bearer or registration token. Submitting a code repeats the
password check. The existing authenticated platform boundary protects enrollment.

Store one credential row per account in PostgreSQL, separate from the profile
record. A pending credential is inactive; its first accepted step and activation
are one write. Login attempts with a code are bounded per resolved account on the
writer, including across app instances and email aliases. The login commit
rechecks the password snapshot and the current credential under the existing
user-row lock. It commits step consumption and the real refresh row together.
No username or password column is written by TOTP. Profile writes must preserve
the same locking and partial-update discipline.

Use the existing locked versions of AWS-LC (`aws-lc-rs` 1.19.0,
`aws-lc-sys` 0.47.0), `data-encoding` 2.10.0, and `zeroize` 1.8.2.
AWS-LC supplies the CSPRNG, randomized-nonce AES-256-GCM, HMAC, and a
constant-time comparison. The adapter implements only RFC 4226 dynamic
truncation and the RFC 6238 bounded step policy, not a new cryptographic
primitive. SHA-1 is restricted to HMAC for authenticator compatibility;
seed confidentiality uses AES-256-GCM. Base32 is library-encoded, and
temporary application buffers carrying key/seed material are zeroized where
practical. This does not promise memory erasure for HTTP or library copies.
Use the already enabled transitive AWS-LC backend and no additional features.

The [versioned AWS-LC source](https://github.com/aws/aws-lc-rs/tree/v1.19.0)
defines the needed APIs. An exact-version check against
[OSV](https://osv.dev/) returned no entries for the four named versions at
selection; this is not a workspace-wide dependency audit or proof of absence
of vulnerabilities.

## Alternatives

- A separate public second-step endpoint and pre-session token would add
  ingress exceptions and new bearer custody. Reusing the password endpoint
  leaves its existing source, request size, and concurrency controls in place.
- `totp-rs` would introduce packages not already resolved, for functionality
  limited here to two time counters and an authenticated seed. Its 5.x check
  otherwise needs an adapter to retain the matched counter. No QR renderer,
  provisioning parser, or provider-specific SDK is needed in IAM.
- A process-local replay cache would allow another replica to accept the same
  code or admit a password-only session after activation. A read replica is
  not authoritative for either decision.
