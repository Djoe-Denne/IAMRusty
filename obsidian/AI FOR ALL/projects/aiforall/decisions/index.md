---
title: "Vague 2 — ADR architecture actuelle"
category: decisions
tags: [architecture, rustycog, platform, visibility/internal]
status: accepted
summary: >-
  Hub : photographie 0100–0502 ; Vague 3 IAM = iamrusty 0407–0410 ;
  Vague 4 cloud 0600–0602 Proposed. Distinctes des ADR Apparatus 0001–0008.
created: 2026-09-12T10:20:00Z
updated: 2026-09-22T14:00:00Z
sources:
  - docs/adr/README.md
  - docs/adr/0100-services-metier-hexagonaux-rustycog.md
  - docs/adr/0200-it-infra-reelle-rustycog-testing.md
  - docs/adr/0300-crates-events-contrat-sans-transport.md
  - docs/adr/0400-iamrusty-identite-hexagonale.md
  - docs/adr/0500-config-typee-et-compose-local.md
  - docs/adr/0600-cloud-portable-opentofu-k8s-gitops.md
  - docs/adr/0601-cluster-trust-namespaces-standalones.md
  - docs/adr/0602-observabilite-portable-otlp-lgtm.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/6de9b375-2f3a-4301-9342-b9a00323c9a8/6de9b375-2f3a-4301-9342-b9a00323c9a8.jsonl
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
---

# Vague 2 — ADR architecture actuelle

Canon git : `docs/adr/README.md`. Ces ADR **photographient le dépôt tel qu’il est** (12 septembre 2026). `Accepted` = cible ratifiée rétroactivement. `Implemented` / `Partial` = réalité du code.

Ne pas confondre avec la [[projects/manifesto/decisions/index|vague 1 Apparatus]] (`0001`–`0008`) : 0002–0008 **Implemented** (Kind V1) sauf **0001 Partial**.

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
| 0600–0699 | Cloud / IaC / GitOps / cluster | [[projects/aiforall/decisions/0600-cloud-portable]], [[projects/aiforall/decisions/0601-cluster-topology]], [[projects/aiforall/decisions/0602-observabilite-portable]] |

Agents Cursor dédiés (Grok 4.6 Extra High) : `adr-hexagonal-rustycog`, `adr-testing-strategy`, `adr-events-authz`, `adr-services-runtime`, `adr-platform-quality` — voir [[projects/aiforall/concepts/orchestrator-agent-harness]].

## Partial connus

- **0202** — files opt-in côté producteurs ; Telegraph IT encore allumée.
- **0301** — outbox same-txn Hive/Manifesto ; IAM txn séparée ; Telegraph sans outbox.
- **0406** — P0 contrats + KV ; Factory / host hors livré (encore Partial) ; moteur P4 = ADR-0008 **Implemented** (Kind V1).
- **0501** — fmt/Clippy/Sonar en place ; gate `new_coverage` 80 % non tenu (~73 %).

**NATS** n’est pas un transport du dépôt (`QueueConfig` = SQS / Kafka / Disabled).

## Vague 3 — IAM living (hors cette photographie)

Les ADR **0407–0410** (IdP fédérés, `Accepted` / Réalité `Partial`) ne photographient pas le dépôt 12 sept. Canon git : `docs/adr/0407`–`0410`. Pointeurs wiki : [[projects/iamrusty/decisions/index]]. Elles **ne font pas** partie de cette vague 2 ; ne pas les fusionner dans [[projects/aiforall/decisions/0400-services-runtime]].

## Vague 4 — Cloud portable (hors cette photographie)

Les ADR **0600–0602** (déploiement portable + topologie cluster + observabilité OTLP, `Proposed` / `Unimplemented`) ne photographient pas le dépôt. Canon git : `docs/adr/0600`–`0602`. Contrat : `docs/platform-cloud-v1-implementation-contract.md`. Pointeurs : [[projects/aiforall/decisions/0600-cloud-portable]], [[projects/aiforall/decisions/0601-cluster-topology]], [[projects/aiforall/decisions/0602-observabilite-portable]].

Elles **ne font pas** partie de cette vague 2 et **ne se fusionnent pas** dans [[projects/aiforall/decisions/0500-plateforme-qualite]] (0500 reste la photo Compose/CI ; l'écart openbao/lazaret/mesh du Compose actuel est **cité** dans 0600/0601, pas « corrigé » ici). **Ne pas** utiliser 0009 / 0411 / 0503. P4 ([[projects/manifesto/decisions/0008-apparatus-p4-k8s]]) s'insère dans cette plateforme ; 0600 **n'est pas** une ADR Apparatus.

| ADR | Décision | Note |
|---|---|---|
| [[projects/aiforall/decisions/0600-cloud-portable]] | Trois couches ; OpenTofu ; GKE (`gcp`) premier adapter ; Flux + Kustomize | living Vague 4 ; Q3 → 0602 |
| [[projects/aiforall/decisions/0601-cluster-topology]] | 4+1 Deployments ; ns `aiforall-*` | ferme le Non décidé K8s de 0404 sans l'éditer |
| [[projects/aiforall/decisions/0602-observabilite-portable]] | Plan A câble rustycog ; plan B LGTM/Tempo derrière collector | ferme Q3 0600 sans SuperSéder 0600 |

## Related

- [[projects/aiforall/aiforall]]
- [[projects/aiforall/decisions/0600-cloud-portable]]
- [[projects/aiforall/decisions/0601-cluster-topology]]
- [[projects/aiforall/decisions/0602-observabilite-portable]]
- [[projects/iamrusty/decisions/index]]
- [[concepts/architecture-coherence-across-services]]
- [[concepts/integration-testing-with-real-infrastructure]]
- [[journal/2026-09-12]]
- [[journal/2026-09-22]]
- [[journal/2026-09-20]]
