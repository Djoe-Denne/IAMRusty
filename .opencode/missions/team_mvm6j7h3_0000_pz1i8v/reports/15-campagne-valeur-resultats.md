# 15 — Campagne de valeur terminée

## Résultat observé
**8 scénarios, 17 appels métier, 14 outils distincts, 32,26 secondes**.
**7 PASS /1 PARTIAL**, pas simplement17 réponses techniques réussies.

[Rapport détaillé et oracles](../../../../docs/missions/20261024-rust-code-mcp-value-results.md).
Plan et sources fixés avant le run ; comparaison des spans/reexports/signatures
aux sources, voisins confirmés avec Serena, contrôle positif/négatif des audits.

## Valeur acquise
Trois vrais consommateurs retrouvés avec positions exactes ; deux vrais voisins
de refactor de transport_status ; pagination complète de deux factories ; quinze
réexports conformes ; seule règle architecturale synthétique positive déclenchée ;
derive audit discriminant. Recherche hybride pertinente top1, 30,218 s froid /
154 ms chaud. Une seule instance8B, pas d'OOM.

## Limite importante, pas masquée par HTTP 200
Codemap à seed explicite exploitable ; prompt seul **PARTIAL** :52 hits perdus à
line-resolve et9nœuds signing IAM hors sujet, cause non établie. Autres réserves :
bruit du dense/getters similaires, références de types incluses dans calls_from,
audits comme déclencheurs de revue plutôt qu'autofix. Aucune correction appliquée.

## Intégrité / retour
Sources989 et plan inchangés, pas de rebuild/index/reset ni d'infrastructure.
Caches libérés, runtime0, VRAM globale libre21819MiB au relevé final (pas un pic).
Preuves privées `value-campaign-20261024.json`, `...-evaluation.json`,
`value-campaign-source-oracles.json`. Même agent pour auteur/validation ; pas de
review indépendante ni de benchmark exhaustif des55 outils.

Recommandation : exploiter search hybride + relations/signatures + codemap
explicite dès maintenant ; investiguer le raccordement sémantique de codemap avant
de lui confier une cartographie autonome à partir d'une phrase. Aucun arbitrage
utilisateur supplémentaire requis pour clôturer cette campagne bornée.
