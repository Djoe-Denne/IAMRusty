---
title: "ADR 0200–0202 — IT réelle, mocks HTTP sortant, files opt-in"
category: decisions
tags: [testing, rustycog, fixtures, visibility/internal]
aliases: [strategie tests, mocks vs infra]
summary: "IT = serveur HTTP + DB + harness rustycog-testing. WireMock = HTTP sortant seulement. OpenFGA = testcontainer. Files producteur opt-in ; Telegraph encore allumé (Partial)."
created: 2026-09-12T10:20:00Z
updated: 2026-09-12T10:20:00Z
sources:
  - docs/adr/0200-it-infra-reelle-rustycog-testing.md
  - docs/adr/0201-mocks-http-sortant-seulement.md
  - docs/adr/0202-transport-opt-in-queues-desactivees.md
  - docs/guides/tests-integration.md
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
---

# ADR 0200–0202 — IT réelle, mocks, files

Canon : `docs/adr/0200`–`0202`. Hub : [[projects/aiforall/decisions/index]]. Détail historique : [[concepts/integration-testing-with-real-infrastructure]] et [[skills/using-rustycog-testing]].

## 0200 — Serveur réel, DB réelle (Implemented)

Toute IT HTTP d’un service métier passe par `setup_test_server()` (`tests/common.rs`). Bootstrap rustycog-testing : Axum réel, Postgres, `Migrator::up/down`, JWT harness (`create_jwt_token`, HS256 `rustycog-test-hs256-secret`). Base URL **déjà préfixée** (`SERVICE_PREFIX`). `#[serial]` dès qu’un singleton process (OpenFGA, WireMock, file) est touché.

Pas de pyramide unitaire chiffrée — hors décision.

**Interdit :** mocker le use-case / handler à la place du serveur.

## 0201 — Mocks = HTTP sortant seulement (Implemented)

| Autorisé | Interdit |
|---|---|
| WireMock OAuth IAM, catalogue Manifesto (`isolated()`), provider Hive | Mocker le use-case HTTP sous test |
| Telegraph `SmtpService` (assert sur la charge émise) | Fake `OpenFgaMockService` / `Check` WireMock |
| MailHog `TestSmtp` (listener SMTP réel) | Pointer `openfga.api_url` sur `:3000` si `has_openfga()` |

OpenFGA en IT = testcontainer `openfga/openfga`, défaut **deny**, `openfga.allow(...)`. IAM : `has_openfga() == false`. Le type `OpenFgaMockService` n’existe plus.

## 0202 — Files opt-in (Partial)

Producteurs (IAM, Hive, Manifesto) : `[queue] enabled = false` + descripteur HTTP `has_sqs() == false`. Opt-in : binaire `sqs_event_routing_tests.rs` + env `*_QUEUE__ENABLED=true`. Kafka = helper optionnel IAM, pas le défaut.

**Écart :** Telegraph consommateur a `enabled = true` et `has_sqs() == true` sur la suite. Ce n’est pas une seconde règle adoptée.

**NATS** : aucune fixture, pas dans `QueueConfig`. IAM `MockEventPublisher` = faux in-process du port events, distinct de l’opt-in SQS.

## Related

- [[projects/rustycog/references/openfga-real-testcontainer-fixture]]
- [[projects/rustycog/references/isolated-wiremock-fixture]]
- [[projects/aiforall/decisions/0300-events-authz]]
