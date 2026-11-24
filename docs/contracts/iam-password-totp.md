# IAM password TOTP

STORY-004 adds optional authenticator enrollment for platform password accounts.
The `/iam` service prefix is included below. Provider sign-in, reset, logout and
existing session lifetimes are unchanged. Enrollment does not revoke sessions
already issued. No public ingress exception is added.

## Enrollment

All three operations require the authenticated platform owner; there is no
request account selector.

- `POST /iam/api/me/totp/enrollment` returns `201` with `enrollment_id` and a
  one-time `otpauth_uri`, with `Cache-Control: no-store`. A second start replaces
  only an incomplete enrollment. The app uses SHA-1 HMAC, six digits, a
  30-second step, and a random 160-bit unpadded Base32 secret. Pending setup
  expires after ten minutes without enabling the factor. Active credentials
  cannot be read or replaced through this API.
- `POST /iam/api/me/totp/enrollment/confirm` takes `enrollment_id`,
  `current_password`, and `code` (a string, including leading zeros). `204`
  means the code was consumed and the factor is active. A bad password or a
  well-formed wrong code returns `401`; malformed input returns `422`. An
  absent, superseded, expired, or cancelled pending enrollment returns `404`
  after current-password proof.
- `DELETE /iam/api/me/totp/enrollment/{enrollment_id}` cancels the current
  matching pending setup with `204`. It cannot remove an active factor.

## Password sign-in

`POST /iam/api/auth/login` keeps the normal unprotected response. After a
valid password and existing login gates, if the factor is active and
`totp_code` is absent, it returns tokenless `401` with
`error.error_code = "totp_required"`. Resubmit `email`, `password`, and
`totp_code` (a string). An invalid or replayed code cannot persist a session
or return credentials. The current or immediately previous step is accepted
at most once, including activation. Wrong passwords do not disclose enrollment.
Five code submissions per resolved enrolled account within a 60-second
window are allowed; further submissions return `429`. A successful login
uses the pre-existing `200` response shape.

## Secret custody and deployment

`totp_key_file` (`IAM_TOTP_KEY_FILE` with the IAM environment loader) is the
path to a deployment-mounted file of exactly 32 random raw bytes. All IAM
instances that serve a writer must use the same dedicated key, separate from
JWT keys and database backups. Do not place key bytes in config, environment
variables, logs, or source control. Missing configuration blocks new
enrollments and makes active-factor sign-in unavailable, not password-only.
An explicitly configured unreadable or malformed key refuses startup.
An incorrect key fails authenticated decryption and cannot yield a session.

Apply the migration before starting the new binary. Once any account enrolls,
neither the old binary nor the migration's destructive `down` is a supported
rollback: they cannot enforce existing factors. The `down` path is for
disposable test database teardown only. Key rotation and deployment to
long-lived environments are not part of STORY-004.
