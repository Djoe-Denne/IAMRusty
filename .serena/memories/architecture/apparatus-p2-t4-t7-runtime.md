# Apparatus P2 T4–T7 runtime — GREEN + §D backoff (2026-09-13)

- Order delivered: T6 ports → T4 apply/CAS → T5 ticker → T7 cleanup+gate → §D backoff writer.
- Ports: `apparatus-contracts` trait sync `ApparatusRuntime` + `RuntimeObservation` (no invoke, no Tokio). Double `InProcessApparatusRuntime` (Mutex+HashMap, `applied=false` on replay). `derived_operation_id` in `Manifesto/infra/src/apparatus_runtime/`.
- Apply: `apply_due_once` claim CAS + bind DTO P0 + `write_observed` fencing (reset retry). Bind fail → `write_bind_failure` fenced. Test hook `apply_due_once_skip_observe`. Constants `APPARATUS_LEASE_TTL=30s`, `APPARATUS_TICK_INTERVAL=2s`, `APPARATUS_RETRY_MAX=8`, backoff base 30s cap 300s.
- Setup: `start_apparatus_runtime` + `is_live()`/`tick_interval` — no P2 tokens in setup/app. Ticker starts in `Application::new` (standalone + monolith). Queue noop ≠ not live.
- Cleanup: `run_once` = due bindings + due jobs; teardown + CAS `completed_at` ; teardown fail → job retry/backoff/terminal (scan excludes terminal).
- Tests: t4 5, t5 10, t6 3, t7 cleanup 4 + gate 5. T1–T7 + readiness 17 EXIT 0. No commit.
