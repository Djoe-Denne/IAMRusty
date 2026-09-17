# Review briefing

```yaml
reviewer: correctness | tests | rust-perf | security
written_at: 2026-09-17T19:00Z
branch: <git branch>
head_sha: <git rev-parse HEAD>
dirty: true | false
scope_slug: <kebab ≤40>
verdict: PASS | GO_WITH_COMMENTS | BLOCK
related_paths:
  - path/to/file.rs
```

## Verdict

Une à trois phrases. BLOCK si un finding l’impose (sécu CRITICAL/HIGH, correctness BLOCKER, etc.).

## Findings

Pour chaque finding (même structure que la sortie reviewer, compressée) :

### <ID> — <SEVERITY>

- **Location:** `fichier:ligne` (symbole si utile)
- **What:** fait observé
- **Why:** conséquence / invariant
- **Suggested fix shape:** forme du correctif (pas un patch complet sauf s’il tient en 10 lignes)
- **Tests to run:** commande `cargo …` ou scénario

S’il n’y a aucun finding : `None.` + ce qui a été vérifié.

## Reasoning

Points de raisonnement **déjà tranchés** (ce que le prochain agent ne doit pas ré-explorer) : chemins publics vs nest, authz hors scope, quel test est la vraie preuve, etc.

## Do not redo

Liste d’enquêtes inutiles. Exemples :

- Ne pas re-prouver que `router()` non préfixé est volontaire (cible `.nest` monolith).
- Ne pas relancer une revue authz T7 si le handler est le même après strip du préfixe.
- Ne pas relire tout ADR-0007 pour un alignement d’index docs.

## Anti-goals

Ce qu’il **ne faut pas** changer (contrats intacts, hors scope, « ne pas inventer »).

## Open questions

Incertitudes restantes. `None` si tout est actionnable.

## Tests

Commandes déjà exécutées (résultat) vs commandes **à** exécuter pour valider le fix.

## Staleness

Ce briefing est lié à `head_sha` / `dirty` ci-dessus. Si le working tree a bougé, revérifier les locations avant d’appliquer.
