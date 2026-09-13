# Apparatus P2 — ADR-0006 Proposed, T1 only (2026-09-12)

- ADR-0006 (`docs/adr/0006-apparatus-p2-reconciliation-in-process.md`) is **Proposed**, not Accepted. T2–T7 (generation, lease, fencing, worker, desired/observed columns) are forbidden until explicit user/PR Accept.
- T1 (allowed by ADR-0001) is GREEN: `source=managed` + `component_status_changed` without event field `binding_id` is ignored; `source=legacy` / missing binding row still apply `project_id + component_type`.
- Code: `Manifesto/infra/src/apparatus_binding_source.rs`, `ComponentStatusProcessor` 2-arg + `SqlApparatusBindingSourceLookup` wired in `Manifesto/setup/src/app.rs` before UoW db move. Event field `binding_id: Option<Uuid>` on `apparatus-events` ComponentStatusChangedEvent (not a SQL column).
- Tests: `cargo test -p manifesto-service --test apparatus_p2_t1_events` (4), `--test apparatus_p2_t1_lookup` (1).
- Do not copy wiki column names. HTTP 202 default NO. No new event type by default.
