# Modèle autorisé pour les agents — décision utilisateur

**Date : 2026-09-16.** Décision utilisateur, vaut pour tous les agents (délégations, sous-agents, workers, review), désormais et pour la suite.

## Slugs autorisés (session Cursor — les seuls)
- `inherit`
- `composer-2.5-fast`
- `cursor-grok-4.6-xhigh`
- `muse-spark-1.3-max`

Il n'existe **pas** de slug `grok-4.6-high` / « Grok 4.6 high ». « high » → `cursor-grok-4.6-xhigh`.

## Politique de mapping
- **Gourmand** (orchestration, architecture, implémentation difficile, reviews sécu/correctness/perf complexes, emergency, ADR, hard-implementer, expert, implementer si jugement) → `cursor-grok-4.6-xhigh`.
- **Moins gourmand** : high n'existe pas → même `cursor-grok-4.6-xhigh`.
- **Lecture** (Explore, recherche, mechanical-worker, cursor-guide, test-reviewer, ci-investigator) → `composer-2.5-fast`.
- `muse-spark-1.3-max` : autorisé dans la liste session, **non assigné** à un agent.
- `inherit` : autorisé en session, **interdit** comme défaut d'agent (chaque agent a un `model` explicite).

## Application (2026-09-16)
- Cursor `.cursor/agents/*.md` : frontmatter `model:` sur les 16 agents.
- Codex `.codex/agents/*.toml` : champ `model =` + consignes « Modèle » alignées (12 agents ; pas de reviewers Codex).
- `mechanical-worker` et `test-reviewer` → Composer. Tous les autres agents du dépôt → Grok 4.6 xhigh.

## Historique
- **Supersede** la décision du **2026-09-15** (GLM 5.3 / GLM Flash uniquement).
- **Supersede** la décision du **2026-09-12** (Grok 4.6 Extra High *seul*, sans Composer lecture). Grok xhigh reste le slug gourmand ; Composer est ajouté pour la lecture.
