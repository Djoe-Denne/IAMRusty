# Paravretius + Manifesto SQS flake (2026-09-18)

- **Paravretius** : 0 hit code/wiki historique. Canon = **Apparatus** / BC P3 **Lazaret**. Page vault `obsidian/AI FOR ALL/entities/paravretius.md` + alias sur `apparatus-platform`.
- T8 `kv_purge` ← `component_removed` (`lazaret-kv-events`) : `f0cf1d2`. T10 enrollment `apparatus_enrollments` landé dans **`9e85edd`** (message de commit trompeur « cursor review briefing »).
- ADR-0007 reste Accepted / Réalité Partial. 0006 G/E inchangés.
- CI run 35266261797 : flake LocalStack `CreateQueue` `dispatch failure` au premier IT SQS alphabétique (`archive_routes_*`). Retry 8× dans `Manifesto/tests/sqs_event_routing_tests.rs`. Pas de bump rustycog. Briefing : `.cursor/review-briefings/20260918T1345Z-correctness-manifesto-sqs-flake-wiki-9e85edd.md` (PASS).
