# Apparatus P2 T3 persist — GREEN (2026-09-13)

- ADR-0006 Accepted + Implemented: J+A+K+H. Create managed inserts `desired_generation=1`, `next_retry_at=NOW()` same txn as ACL + rustycog outbox. Delete (before CASCADE) inserts `apparatus_cleanup_jobs` if managed AND (digest IS NOT NULL OR desired_generation > 0).
- SQL shared in `Manifesto/infra/src/apparatus_outbox.rs`: `insert_managed_binding`, `insert_cleanup_job`, `load_binding_cleanup_row`, `persist_cleanup_job_atomically`, `bump_managed_desired_generation` (CAS +1, 1 retry). UoW `save_component_with_events` inserts on create and bumps on update (`!create_acl`). Delete still snapshots into `apparatus_cleanup_jobs` (no bump) — décision figée 2026-09-13, pas un écart.
- `transaction.rs` and `apparatus_outbox.rs` remain **outside** T7 allowlist: no tokens poll/worker/lease/fencing/controller/desired_state. Backoff SQL is only in `apparatus_runtime/`.
- Tests: `Manifesto/tests/apparatus_p2_t3_persist.rs` (7). Update managed PATCH ⇒ gen 2 then 3 ; legacy stays 0.
