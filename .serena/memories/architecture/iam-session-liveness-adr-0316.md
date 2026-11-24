# IAM session liveness — ADR 0316

Status: Proposed / Unimplemented. See `docs/adr/0316-password-recovery-session-liveness.md` and its referenced contract.

- Password recovery would advance a durable platform-account epoch atomically with password update and refresh-token purge.
- Direct and mesh admission would check the epoch against writer authority; unavailable liveness would fail closed.
- Issuance and refresh fences must preserve the pre/post-reset ordering; signed-in password change remains out of scope.
- Product and architecture approvals, implementation, and behavioral proof are pending.