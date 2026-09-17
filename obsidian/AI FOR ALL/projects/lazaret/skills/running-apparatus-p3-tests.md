---
title: >-
  Lancer les tests Apparatus P3
category: skills
tags: [testing, rust, components]
sources:
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - docs/apparatus-p3-implementation-prompt.md
  - Lazaret/tests/apparatus_p3_t3_identity.rs
  - Lazaret/tests/apparatus_p3_t7_invoke.rs
  - Lazaret/tests/common.rs
summary: >-
  Preuves P3 sous Lazaret/tests/apparatus_p3_t*.rs : identité, grants,
  consentement, KV, invoke, T8 kv_purge, T9 prefix invoke.
  ADR-0007 Accepted / Partial.
provenance:
  extracted: 0.90
  inferred: 0.10
  ambiguous: 0.00
created: 2026-09-17T10:55:00Z
updated: 2026-09-17T17:40:00Z
---

# Lancer les tests Apparatus P3

ADR-0007 Accepted, Réalité **Partial**. Hub : [[projects/lazaret/lazaret]]. Concept : [[projects/lazaret/concepts/workload-identity]]. P2 reste dans Manifesto : [[projects/aiforall/skills/running-apparatus-p2-tests]].

TDD 13–15 sept. : Phase 0 + T1 seulement tant que l’ADR était Proposed ; T2–T7 après Accept explicite. Rien n’a été auto-Accepté. ^[inferred]

## Commandes

```text
cargo test -p lazaret-service --test apparatus_p3_t3_identity -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t4_grants -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t5_consent -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t6_kv -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t7_invoke -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t8_kv_purge -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t9_invoke_prefix -- --test-threads=1
cargo test -p lazaret-http
cargo test -p manifesto-service --test sqs_event_routing_tests -- --test-threads=1
cargo test -p lazaret-service --test health
```

Fixtures : snapshot de binding (WireMock Manifesto), Redis, Vault KV v2. Harness `Lazaret/tests/common.rs`. Prefer `--test-threads=1` dès qu’il y a Postgres / Redis / Vault.

## Related

- [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]
- [[projects/aiforall/concepts/orchestrator-agent-harness]]
