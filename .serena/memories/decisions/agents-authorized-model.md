# Modèle autorisé pour les agents — décision utilisateur

**Date : 2026-09-23.** Décision utilisateur (Grok 4.7 pour hard-implementer + petit lot haute-raisonnement). Vaut pour les agents Cursor du dépôt.

## Slugs autorisés (session Cursor — les seuls à assigner)
- `inherit` — autorisé en session, **interdit** comme défaut d'agent (chaque agent a un `model` explicite).
- `composer-2.5-fast` — lecture / mécanique.
- `cursor-grok-4.6-xhigh` — gourmand par défaut (orchestrateur, implementer, emergency, ADR, correctness, rust-perf).
- `grok-4.7-xhigh` — **seul** slug Grok 4.7 ; pas de `grok-4.7-high` ni autre variante inventée.
- `muse-spark-1.3-max` — autorisé en session, **non assigné** à un agent.
- `claude-opus-5-5-medium` — peut figurer dans la liste session ; **ne pas l'assigner** sauf demande utilisateur explicite.

Il n'existe **pas** de slug `grok-4.6-high` / « Grok 4.6 high ». « high » 4.6 → `cursor-grok-4.6-xhigh`.
Il n'existe **pas** de slug `grok-4.7-high`. Grok 4.7 → `grok-4.7-xhigh` uniquement.

## Politique de mapping (2026-09-23)
- **Haute caution / haute raisonnement** → `grok-4.7-xhigh` :
  - `hard-implementer` (obligatoire demandé)
  - `architecte` (architecture vivante)
  - `expert-engineer` (expertise / challenge)
  - `security-reviewer` (revue sécu / red-team)
- **Gourmand restant** → `cursor-grok-4.6-xhigh` : orchestrateur, implementer, emergency-engineer, adr-*, correctness-reviewer, rust-perf-reviewer.
- **Lecture** → `composer-2.5-fast` : Explore, mechanical-worker, test-reviewer, cursor-guide, ci-investigator.
- Orchestrateur : **reste** sur `cursor-grok-4.6-xhigh` jusqu'à preuve de 4.7 sur hard-implementer. Prochaine vague possible : orchestrateur seul, pas blanket.
- En lançant un agent 4.7 via Task, passer `model: grok-4.7-xhigh` (ne pas héritier du parent 4.6).

## Application Cursor (2026-09-23)
- `.cursor/agents/hard-implementer.md`, `architecte.md`, `expert-engineer.md`, `security-reviewer.md` : `model: grok-4.7-xhigh`.
- `.cursor/agents/orchestrator.md` : routage Model priority mis à jour ; **son propre** `model:` reste `cursor-grok-4.6-xhigh`.
- Codex `.codex/agents/*.toml` : **non modifié** ce tour (périmètre Cursor).

## Historique
- **Supersede** la décision du **2026-09-16** (tout le gourmand sur `cursor-grok-4.6-xhigh`, lecture Composer).
- **Supersede** la décision du **2026-09-15** (GLM 5.3 / GLM Flash uniquement).
- **Supersede** la décision du **2026-09-12** (Grok 4.6 Extra High *seul*).
