---
title: "Vague 2 — ADR architecture actuelle"
category: decisions
tags: [architecture, rustycog, platform, visibility/internal]
status: accepted
summary: "Hub des ADR rétroactives 0100–0502 : hexagone, tests, events/authz, services, plateforme. Distinctes des ADR Apparatus 0001–0005."
created: 2026-09-12T10:20:00Z
updated: 2026-09-12T10:20:00Z
sources:
  - docs/adr/README.md
  - docs/adr/0100-services-metier-hexagonaux-rustycog.md
  - docs/adr/0200-it-infra-reelle-rustycog-testing.md
  - docs/adr/0300-crates-events-contrat-sans-transport.md
  - docs/adr/0400-iamrusty-identite-hexagonale.md
  - docs/adr/0500-config-typee-et-compose-local.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/6de9b375-2f3a-4301-9342-b9a00323c9a8/6de9b375-2f3a-4301-9342-b9a00323c9a8.jsonl
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
---

# Vague 2 — ADR architecture actuelle

Canon git : `docs/adr/README.md`. Ces ADR **photographient le dépôt tel qu’il est** (12 septembre 2026). `Accepted` = cible ratifiée rétroactivement. `Implemented` / `Partial` = réalité du code.

Ne pas confondre avec la [[projects/manifesto/decisions/index|vague 1 Apparatus]] (`0001`–`0005`) : cibles de plateforme, souvent encore `Partial`.

## Numérotation par plages

Pas un compteur global. Un sujet hexagonal n’est **pas** `0006`.

| Plage | Sujet | Page wiki |
|---|---|---|
| 0001–0099 | Apparatus | [[projects/manifesto/decisions/index]] |
| 0100–0199 | Hexagone / crates | [[projects/aiforall/decisions/0100-hexagone-rustycog]] |
| 0200–0299 | Tests IT, mocks, files | [[projects/aiforall/decisions/0200-strategie-tests]] |
| 0300–0399 | Events, outbox, AuthN/AuthZ | [[projects/aiforall/decisions/0300-events-authz]] |
| 0400–0499 | Services et runtimes | [[projects/aiforall/decisions/0400-services-runtime]] |
| 0500–0599 | Config, CI, rustycog | [[projects/aiforall/decisions/0500-plateforme-qualite]] |

Agents Cursor dédiés (Grok 4.6 Extra High) : `adr-hexagonal-rustycog`, `adr-testing-strategy`, `adr-events-authz`, `adr-services-runtime`, `adr-platform-quality` — voir [[projects/aiforall/concepts/orchestrator-agent-harness]].

## Partial connus

- **0202** — files opt-in côté producteurs ; Telegraph IT encore allumée.
- **0301** — outbox same-txn Hive/Manifesto ; IAM txn séparée ; Telegraph sans outbox.
- **0406** — P0 contrats + KV ; Factory / host / gateway hors livré.
- **0501** — fmt/Clippy/Sonar en place ; gate `new_coverage` 80 % non tenu (~73 %).

**NATS** n’est pas un transport du dépôt (`QueueConfig` = SQS / Kafka / Disabled).

## Related

- [[projects/aiforall/aiforall]]
- [[concepts/architecture-coherence-across-services]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[journal/2026-09-12]]
