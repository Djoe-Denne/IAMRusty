---
title: >-
  Agent Architecte
category: concepts
tags: [architecture, visibility/internal]
aliases: [architecte]
sources:
  - .cursor/agents/architecte.md
  - .codex/agents/architecte.toml
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/804c422a-63c7-42d5-94a0-dba53752c278/804c422a-63c7-42d5-94a0-dba53752c278.jsonl
summary: >-
  Architecture vivante : impact, contradictions ADR/code/docs, ADR Proposed,
  contrat implementer. Pas de code applicatif. Pas les ADR 0100–0502.
provenance:
  extracted: 0.84
  inferred: 0.14
  ambiguous: 0.02
created: 2026-09-17T10:55:00Z
updated: 2026-09-17T10:55:00Z
---

# Agent Architecte

Installé le 13 sept. 2026 (`666ead4`, session `804c422a`). Dérivé du dépôt (canon `docs/adr/`, plages, agents `adr-*`, harnais cheap-first), pas d’un template générique. Harnais : [[projects/aiforall/concepts/orchestrator-agent-harness]]. Hub : [[projects/aiforall/aiforall]].

## Quand

Impact de frontière (service, protocole, stockage, API, sécu structurante). Contradiction ADR ↔ code ↔ docs/wiki. Faut-il une **nouvelle** ADR, une maj vivante, ou aucune. Contrat d’implémentation **avant** le coding agent.

Pas pour : bug local, rename, scaffold déjà spécifié, revue sécu/Bugbot, génération **0100–0502** (agents `adr-*`).

## Autorité

1. Demande utilisateur explicite
2. Règles du dépôt
3. ADR `docs/adr/` = cible (`Statut`) ; `Réalité` = ce que le code fait
4. Code = comportement réel
5. Docs / wiki = annoncé / conception

Désaccord = signaler les chemins. Ne pas « réparer » le code ni lisser l’ADR. Une note wiki `status: proposed` n’est pas une ADR `Accepted`. L’Architecte **propose** ; il **n’Accept pas** tout seul.

Wiki = pointeur « Canon : … ». Memories Serena `.serena/memories/architecture/` = digest, pas canon. Interdit de laisser un digest `*-proposed` une fois `Statut: Accepted`.

## Related

- [[projects/manifesto/decisions/index]]
- [[projects/aiforall/decisions/index]]
- [[journal/2026-09-17]]
