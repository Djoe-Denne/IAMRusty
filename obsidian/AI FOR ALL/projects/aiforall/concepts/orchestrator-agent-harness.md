---
title: >-
  Harnais orchestrator Cursor
category: concepts
tags: [architecture, rust, visibility/internal]
aliases: [agent-harness, orchestrator]
sources:
  - .cursor/rules/00-agent-harness.mdc
  - .cursor/agents/orchestrator.md
  - .serena/memories/constraints/no-muse-spark.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d0c9b007-93a2-4cec-814a-155dcbb74d59/d0c9b007-93a2-4cec-814a-155dcbb74d59.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/e32d072c-e2be-42a4-82f4-2b6808695124/e32d072c-e2be-42a4-82f4-2b6808695124.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/6de9b375-2f3a-4301-9342-b9a00323c9a8/6de9b375-2f3a-4301-9342-b9a00323c9a8.jsonl
  - .cursor/agents/adr-hexagonal-rustycog.md
  - .cursor/agents/architecte.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/804c422a-63c7-42d5-94a0-dba53752c278/804c422a-63c7-42d5-94a0-dba53752c278.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/a340bab3-a47f-4710-af1e-3d4bff06e4a2/a340bab3-a47f-4710-af1e-3d4bff06e4a2.jsonl
  - .cursor/review-briefings/README.md
summary: >-
  Toute tâche projet passe par orchestrator. Reviewers persistent sous
  .cursor/review-briefings/. Grok xhigh ; Composer lecture-only.
provenance:
  extracted: 0.86
  inferred: 0.14
  ambiguous: 0.00
created: 2026-09-11T05:45:00Z
updated: 2026-09-22T06:55:00Z
---

# Harnais orchestrator Cursor

Installé le 10 septembre 2026 (commit `a72979a`). Règle always-on : `.cursor/rules/00-agent-harness.mdc`.

## Contrat

- Pour toute demande projet (code, plan, investigation, reco technique), le root **délègue** la requête complète à `orchestrator`.
- Le root ne lance pas Explore / Bash / implementer lui-même. Seul l’orchestrateur choisit les workers.
- Exception : messages purement conversationnels hors projet.

## Workers

| Type | Rôle |
|---|---|
| `orchestrator` | Plan, architecture, délégation, conformité, jugement final |
| `implementer` | Implémentation quotidienne une fois le design figé |
| `mechanical-worker` | Renames, boilerplate, edits déterministes |
| `hard-implementer` | Implémentation difficile (traits, cross-subsystem) |
| `expert-engineer` | Architecture / reviews coûteuses |
| `emergency-engineer` | Last resort, jamais auto |
| `adr-hexagonal-rustycog` | ADR plage 0100 (hexagone / crates) |
| `adr-testing-strategy` | ADR plage 0200 (IT, mocks, files) |
| `adr-events-authz` | ADR plage 0300 (events, outbox, JWT/FGA) |
| `adr-services-runtime` | ADR plage 0400 (services, dual runtime) |
| `adr-platform-quality` | ADR plage 0500 (config, CI, rustycog) |
| `architecte` | Architecture vivante, ADR Proposed, contrat — [[projects/aiforall/concepts/architecte-agent]] |
| `correctness-reviewer` | Correctness fonctionnelle (read-only) |
| `test-reviewer` | Couverture des risques par les tests |
| `rust-perf-reviewer` | Perf / ownership Rust |
| `security-reviewer` | Authn/authz, frontières Apparatus |

Ne pas précharger `.cursor/agents/*.md` ; le registre Cursor suffit. Les prompts détaillés se chargent à l’invocation. Hub ADR : [[projects/aiforall/decisions/index]].

## Review briefings persistants (depuis 17 sept.)

Une revue dans le chat disparaît au tour suivant. Convention : les reviewers (`correctness`, `tests`, `rust-perf`, `security`) persistent un briefing sous `.cursor/review-briefings/` après **chaque** revue (PASS, commentaires, BLOCK) et tiennent `INDEX.md`. Fichiers **gitignorés** sauf `README.md` + `TEMPLATE.md`. Si le reviewer est en lecture seule, l’**orchestrateur** écrit le fichier. Avant un fix : lire INDEX + briefings du scope et les coller dans le work package. Les implementers lisent les briefings matching **avant** d’explorer. Un compte-rendu chat n’est pas suffisant. Un briefing stale (`head_sha` ou fichiers bougés) est une piste, pas une parole d’évangile.

## Modèles

La session du 10 sept. a remplacé des occurrences Muse trop coûteuses par **Grok 4.6 Extra High** pour le travail courant. Muse reste utilisable quand on le demande explicitement (ex. triple review P0). ^[inferred]

Le 16 sept., le slug `High` n’existe pas dans la liste sous-agents → fallback **`cursor-grok-4.6-xhigh`**. `composer-2.5-fast` est réservé à la lecture. Une passe `glm-5p3` a été posée sur 12 agents puis le frontmatter Architecte est revenu à Grok. ^[ambiguous]

Le 12–13 sept., P2 a été mené sous ce harnais (TDD T1–T7, aujourd’hui Implemented). Agents `adr-*` aussi versionnés `.codex/agents/*.toml` (`5991670`). P3 Lazaret (13–16 sept.) idem. P4-core (21–22 sept.) : Kind `apparatus-p4-it` ; orchestrateur parfois abort alors que les workers ont déjà écrit — reprendre le lot, ne pas relivrer T1. [[journal/2026-09-22]]

## Patterns durables (P1, 12 sept.)

- **Duel gather** : deux agents isolés (agent + contre-agent, contexte minimal, GATHER ONLY) sur la même tâche, puis un résolveur produit une résolution unique avant tout code.
- **Résolution unique** : conventions DB/AuthZ/HTTP figées par écrit, divergences tranchées, escalades listées — source de vérité des tranches TDD.
- **Reviews docs** : rédacteur → reviewer → contre-reviewer indépendant → fix mécaniques ; `Accepted` intact, compteurs vérifiés (P1 : 36/42/78).

## Related

- [[projects/aiforall/aiforall]]
- [[projects/manifesto/concepts/apparatus-p0-contracts]] — première livraison menée sous ce harnais
- [[projects/manifesto/concepts/apparatus-p1-persistence]] — duel gather + tranches TDD sous ce harnais
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]] — TDD T1–T7 sous ce harnais
- [[projects/lazaret/lazaret]] — P3 sous ce harnais
- [[projects/aiforall/concepts/architecte-agent]]
- [[journal/2026-09-20]]
- [[journal/2026-09-17]]
- [[journal/2026-09-13]]
- [[journal/2026-09-12]]
