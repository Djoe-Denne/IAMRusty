# IAM provider intents — ADR 0414

Status: Proposed / Unimplemented. See `docs/adr/0414-oauth-provider-intents.md` and its referenced contract.

- Persist separate Login, Signup, and Link intents; retain OAuthConnect and ADR 0412's browser-binding protocol.
- Resolve Login by provider instance and opaque subject only; email is not identity or link authority.
- Signup would create a complete account and its first session at confirmation; Link remains separately authenticated.
- Cutover and contract decisions are open; this proposal does not authorize implementation.