# Apparatus P2 — ADR-0006 Accepted (2026-09-12)

- User « Je te fais confiance » = Accept. Checklist A–M frozen in docs/adr/0006-apparatus-p2-reconciliation-in-process.md.
- Generation: command CAS desired_generation; worker never increments. Lease 30s per binding, steal on expiry, no heartbeat. Fencing = desired_generation + lease_epoch.
- SQL: 9 additive columns on apparatus_bindings + apparatus_cleanup_jobs (no FK). HTTP 202 NO. No K8s. No new event type. Consent hors P2.
- Ports: ApparatusRuntime bind/configure/unbind/observe/teardown (sync). InProcessApparatusRuntime.
- T1–T7 tests green (t1 4+1, t2 12, t3 5, t4 4, t5 5, t6 3, t7 2+5). T7 P1 still 3.
- Écarts de réalité P2 (cible Accepted inchangée) : §A pas de bump update/remove (create pose desired_generation=1 + next_retry_at=NOW(), génération figée à 1 ; cf. `mem:architecture/apparatus-p2-t3-persist`). §D colonnes retry présentes, aucun writer prod, next_retry_at à l'insert, échec bind = re-scan sans backoff. §I /ready not wired to ticker (is_live() explicit).
