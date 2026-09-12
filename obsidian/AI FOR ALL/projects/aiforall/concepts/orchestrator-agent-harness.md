---
title: "Harnais orchestrator Cursor"
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
summary: >-
  Toute tâche projet passe par orchestrator ; Grok-only. Agents adr-* pour générer les ADR rétroactives.
provenance:
  extracted: 0.86
  inferred: 0.14
  ambiguous: 0.00
created: 2026-09-11T05:45:00Z
updated: 2026-09-12T10:20:00Z
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

Ne pas précharger `.cursor/agents/*.md` ; le registre Cursor suffit. Les prompts détaillés se chargent à l’invocation. Hub ADR : [[projects/aiforall/decisions/index]].

## Modèles

La session du 10 sept. a remplacé des occurrences Muse trop coûteuses par **Grok 4.6 Extra High** pour le travail courant. Muse reste utilisable quand on le demande explicitement (ex. triple review P0). ^[inferred]

Le 12 sept., l'utilisateur a d'abord priorisé **Muse Spark Max** premier / Grok second (7 fichiers agents mis à jour), puis a **interdit Muse Spark** pour toute délégation (`Plus le droit d'utiliser Muse Spark`, mémoire `constraints/no-muse-spark`). État effectif : les 6 agents Cursor sont en `model: cursor-grok-4.6-xhigh`, et seul Grok 4.6 Extra High est autorisé (résolveurs, reviews et contre-agents inclus). Le prompt P2 en préparation doit l'exiger explicitement en tête. ^[inferred]

## Patterns durables (P1, 12 sept.)

- **Duel gather** : deux agents isolés (agent + contre-agent, contexte minimal, GATHER ONLY) sur la même tâche, puis un résolveur produit une résolution unique avant tout code.
- **Résolution unique** : conventions DB/AuthZ/HTTP figées par écrit, divergences tranchées, escalades listées — source de vérité des tranches TDD.
- **Reviews docs** : rédacteur → reviewer → contre-reviewer indépendant → fix mécaniques ; `Accepted` intact, compteurs vérifiés (P1 : 36/42/78).

## Related

- [[projects/aiforall/aiforall]]
- [[projects/manifesto/concepts/apparatus-p0-contracts]] — première livraison menée sous ce harnais
- [[projects/manifesto/concepts/apparatus-p1-persistence]] — duel gather + tranches TDD sous ce harnais
- [[journal/2026-09-11]]
- [[journal/2026-09-12]]
