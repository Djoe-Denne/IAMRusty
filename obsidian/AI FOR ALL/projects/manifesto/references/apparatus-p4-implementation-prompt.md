---
title: >-
  Prompt d’implémentation Apparatus P4
category: references
tags: [architecture, testing, visibility/internal]
sources:
  - docs/apparatus-p4-implementation-prompt.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/apparatus-p4-core-implementation-prompt.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733.jsonl
summary: >-
  Contrat TDD P4 : APP-01 tranché, ADR-0008 Accepted / Partial.
  P4-core T2–T12 livré dans apparatus-operator. T1 absence déjà là.
  Pas Implemented (Transit-dev, Adm-B).
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-20T15:55:00Z
updated: 2026-09-22T10:00:00Z
---

# Prompt d’implémentation Apparatus P4

Source : `docs/apparatus-p4-implementation-prompt.md` et `docs/apparatus-p4-core-implementation-prompt.md` — prompts **POST-LOT historiques** ; **ne pas** relivrer T2–T12. Canon : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]]. Plan conception : [[projects/manifesto/references/apparatus-implementation-plan]].

## État

- **APP-01 tranché.** ADR-0008 **Accepted** / Réalité **Partial**.
- Phase 0 **faite**. T1 absence (`Manifesto/tests/apparatus_p4_t1_absence.rs`) **déjà livrée** — ne pas relivrer.
- T2–T12 **P4-core livrés** dans [[projects/manifesto/concepts/apparatus-p4-operator]] (2026-09-21/22). Ne pas flipper 0008 en Implemented.
- Slice-1 laisse 0002 / 0003 / 0005 **Partial**. Interdit SuperSède 0003/0005.

## Gel A-DEC (ne pas rouvrir)

[[projects/manifesto/references/0007-closeout]] : APP-05 ouvert ; G/E restent ; K8s-**as-P3** interdit ; pas de 2ᵉ protocole ; `invoke` = Lazaret. Gates Manifesto (`k8s`/`kubernetes` sous `Manifesto/*/src`) **non retargetées**.

Skill `aiforall-new-service` **non** (BC-A, pas un 6ᵉ hexagone HTTP). Pas de répertoire `Factory/`. Pas de `KubernetesAdapter` dans Manifesto.

## Related

- [[projects/manifesto/references/0008-app01-reconciliation]]
- [[projects/manifesto/decisions/0005-apparatus-protocol]]
- [[projects/manifesto/decisions/0003-apparatus-untrusted]]
- [[projects/manifesto/concepts/apparatus-p4-operator]]
- [[projects/aiforall/skills/running-apparatus-p4-tests]]
- [[journal/2026-09-22]]
