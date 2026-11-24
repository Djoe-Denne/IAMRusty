# IAM saved-address ownership verification (STORY-008)

This backend slice records ownership proof for an existing saved address.
It does not change sign-in eligibility, signup verification, password recovery,
OAuth linking, primary designation, or which addresses are saved on an account.
Profile UI and browser sign-in continuation are not shipped here.

## API

All routes below are relative to `/iam` and require a normal platform human
bearer session. The server derives the account from that session.

- `GET /api/me/email-addresses` returns `200` and an array of the account's
  `{id,email,is_primary,is_verified}` entries. The status combines legacy
  verification with profile ownership proof for the current exact saved row.
  `/api/me` and the legacy flag are unchanged.
- `POST /api/me/email-verification/request` takes `{"address_id":"<uuid>"}`.
  `202` means the verification message was queued to the stored address, not
  that SMTP delivered it. Missing, foreign and already verified rows all
  produce `404`. Another explicit request inside the 60-second cooldown for
  that address gets `429` and does not extend the cooldown. An accepted
  replacement invalidates the previous link for that address only.
- `POST /api/me/email-verification/confirm` takes `{"token":"<capability>"}`.
  `204` means the link was consumed and proof recorded atomically. Unknown,
  expired, used, superseded, changed-address and wrong-account links all
  produce the same empty `404`. Failure cannot consume a link, send mail or
  establish a session. A GET is never a confirmation.

## Delivery

`IAM_PROFILE_EMAIL_VERIFICATION__LANDING_URL` must be an explicitly configured
absolute HTTPS URL without userinfo, query or fragment. If unset, issuance
returns `503`; IAM does not infer an origin from a request. The message link
adds the opaque random capability as its entire fragment, with no account or
address in the URL. The future landing must do nothing on open, retain the
fragment locally during ordinary sign-in, and send it only on explicit
confirmation after sign-in. It must never pass the capability to an identity
provider. The link expires after 24 hours and must not be logged.

IAM stores only a digest in the capability record. The raw fragment is present
in the private outbox and mail transport as part of the delivery URL. Make the
additive `profile_email_verification_requested` v1 route available to
Telegraph before enabling issuance. Outbox and SMTP are asynchronous; the
contract does not promise exactly one transport attempt. A message delayed
past expiration or supersession carries an unusable link.
