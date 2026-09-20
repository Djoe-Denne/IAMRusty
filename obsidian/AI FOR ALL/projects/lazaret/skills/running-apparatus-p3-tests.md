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
  - Lazaret/tests/apparatus_p3_t11b_session_mtls.rs
  - Lazaret/tests/apparatus_p3_t12_openbao.rs
  - Lazaret/tests/apparatus_p3_t13_ca_persist.rs
  - Hive/tests/https_mesh_optional_mtls.rs
  - Lazaret/tests/common.rs
summary: >-
  Preuves P3 T3–T13 sous Lazaret/tests ; T14b https_mesh_optional_mtls
  Hive/IAM/Telegraph. ADR-0007 Accepted / Partial.
provenance:
  extracted: 0.92
  inferred: 0.08
  ambiguous: 0.00
created: 2026-09-17T10:55:00Z
updated: 2026-09-20T10:35:00Z
---

# Lancer les tests Apparatus P3

ADR-0007 Accepted, Réalité **Partial**. Hub : [[projects/lazaret/lazaret]]. Concept : [[projects/lazaret/concepts/workload-identity]]. Mesh : [[projects/aiforall/concepts/https-platform-mesh]]. P2 reste dans Manifesto : [[projects/aiforall/skills/running-apparatus-p2-tests]].

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
cargo test -p lazaret-service --test apparatus_p3_t10_enrollment_persist -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t11b_session_mtls -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t12_openbao -- --test-threads=1
cargo test -p lazaret-service --test apparatus_p3_t13_ca_persist -- --test-threads=1
cargo test -p hive-service --test https_mesh_optional_mtls -- --test-threads=1
cargo test -p iam-service --test https_mesh_optional_mtls -- --test-threads=1
cargo test -p telegraph-service --test https_mesh_optional_mtls -- --test-threads=1
cargo test -p lazaret-http
cargo test -p manifesto-service --test sqs_event_routing_tests -- --test-threads=1
cargo test -p lazaret-service --test health
```

Fixtures : snapshot de binding (WireMock Manifesto), Redis, Vault/OpenBao. T6 KV **reste** wiremock (preuve protocole). T12 OpenBao = testcontainer `lazaret_test-openbao` (preuve produit). T14b = `https_mesh_optional_mtls` sur Hive, IAMRusty, Telegraph. Harness `Lazaret/tests/common.rs`. Prefer `--test-threads=1` dès qu’il y a Postgres / Redis / Vault / OpenBao / LocalStack.

IT SQS Manifesto : retry `CreateQueue` LocalStack (flake hyper dispatch, pas une régression `src`). Voir [[concepts/integration-testing-with-real-infrastructure]].

## Related

- [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]
- [[projects/aiforall/concepts/orchestrator-agent-harness]]
- [[journal/2026-09-20]]
