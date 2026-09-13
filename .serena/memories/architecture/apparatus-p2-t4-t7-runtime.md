# Apparatus P2 T4–T7 runtime — GREEN (2026-09-12)

- Order delivered: T6 ports → T4 apply/CAS → T5 ticker → T7 cleanup+gate.
- Ports: `apparatus-contracts` trait sync `ApparatusRuntime` + `RuntimeObservation` (no invoke, no Tokio). Double `InProcessApparatusRuntime` (Mutex+HashMap, `applied=false` on replay). `derived_operation_id` in `Manifesto/infra/src/apparatus_runtime/`.
- Apply: `apply_due_once` claim CAS + bind DTO P0 + `write_observed` fencing. Test hook `apply_due_once_skip_observe`. Constants `APPARATUS_LEASE_TTL=30s`, `APPARATUS_TICK_INTERVAL=2s`.
- Setup: `start_apparatus_runtime` + `is_live()`/`tick_interval` — no P2 tokens in setup/app. Ticker starts in `Application::new` (standalone + monolith). Queue noop ≠ not live.
- Cleanup: `run_once` = due bindings + due jobs; teardown + CAS `completed_at`. Gate P3+ in `apparatus_p2_t7_gate.rs`; P1 T7 keeps P2 allowlist L.
- Tests listed in the P2 prompt all EXIT 0. No commit.
