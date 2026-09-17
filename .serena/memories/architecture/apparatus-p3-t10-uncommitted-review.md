# Revue uncommitted T10 (2026-09-17)

FULL REVIEW working tree enrollment persist. Briefings gitignorés : `.cursor/review-briefings/INDEX.md` + `20260917T1910Z-*-uncommitted-p3-t10-1472ab5.md`.

Verdicts : correctness PASS ; tests / rust-perf / security GO_WITH_COMMENTS. Pas de BLOCK.

Corrigés : T-1 fail-closed revoke Err sans purge KV ; T-2 bump grant_revision → 409 session figée rev 0 ; T-3 put même empreinte no-op Postgres ; S-1 `binding_enrolled` → Result (erreur store = 500, pas de signature CA) ; R-1 `EnrollmentStore` async, plus de `run_sync`/`block_in_place`.

Laissés : R-2 (2 RTT enroll, risque first-wins) ; R-3 MEASURE alloc Statement ; vault `..` hors diff.

Preuve : `cargo test -p lazaret-service --test apparatus_p3_t10_enrollment_persist --test apparatus_p3_t3_identity --test apparatus_p3_t8_kv_purge -- --test-threads=1` → 18 passed. Pas de commit.

Ne pas relitiger ADR-0007, first-wins 409, router nu non préfixé. Voir `mem:architecture/apparatus-p3-review-fixes` (lot 2026-09-16).