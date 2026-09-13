# Apparatus P2 — ADR-0006 Accepted + Implemented (2026-09-13)

- User « Je te fais confiance » = Accept. Checklist A–M frozen in docs/adr/0006-apparatus-p2-reconciliation-in-process.md.
- Generation: command CAS desired_generation; worker never increments. Lease 30s per binding, steal on expiry, no heartbeat. Fencing = desired_generation + lease_epoch.
- SQL: 9 additive columns on apparatus_bindings + apparatus_cleanup_jobs (no FK). HTTP 202 NO. No K8s. No new event type. Consent hors P2.
- Ports: ApparatusRuntime bind/configure/unbind/observe/teardown (sync). InProcessApparatusRuntime.
- T1–T7 + §D backoff 2026-09-13 : t1 4+1, t2 12, t3 7, t4 5, t5 10, t6 3, t7 4+5, readiness 17. T7 P1 still 3.
- §I fermé : `/ready` check `apparatus_runtime` (`live_flag` + Drop guard). Queue disabled ≠ not_ready.
- §A update CAS +1 livré (PATCH status). Delete/remove ne bump pas (snapshot + cleanup) — décision figée, pas un écart.
- §D writer livré : APPARATUS_RETRY_MAX=8, backoff min(30s*2^retry_count, 5min), codes bind_failed/teardown_failed, terminal exclu du scan. Réalité Implemented. Aucun écart P2 ouvert. P3–P6 hors scope.
