# Apparatus P2 T3 persist — GREEN (2026-09-12)

- ADR-0006 Accepted: J+A+K+H. Create managed inserts `desired_generation=1`, `next_retry_at=NOW()` same txn as ACL + rustycog outbox. Delete (before CASCADE) inserts `apparatus_cleanup_jobs` if managed AND (digest IS NOT NULL OR desired_generation > 0).
- SQL shared in `Manifesto/infra/src/apparatus_outbox.rs`: `insert_managed_binding`, `insert_cleanup_job`, `load_binding_cleanup_row`, `persist_cleanup_job_atomically`. UoW `save_component_with_events` / `delete_component_with_events` reuse these on the existing txn (no nested txn).
- `transaction.rs` and `apparatus_outbox.rs` remain **outside** T7 allowlist: no tokens poll/worker/lease/fencing/controller/desired_state.
- Tests: `Manifesto/tests/apparatus_p2_t3_persist.rs` (5). T1/T2/T4/T7 stay green. No new FGA type, no new event type, no commit.
- Out of T3: bump `desired_generation` on update/remove (ADR A CAS +1) — not delivered here.