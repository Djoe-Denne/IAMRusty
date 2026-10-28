# 2 — Cause vérifiée et correction de disponibilité

## Objectif / étape
Rendre les appels MCP utilisables sans attendre les générations RPG initiales.
Le profil emergency-thinker est maintenant actif par handoff de la session lead,
sans changement de modèle ni création de worker.

## Cause vérifiée
Le log `C:/Users/djden/.config/1mcp/projects/aiforall/grepai-state/logs/grepai-watch.log`
du 2026-10-23 17:59:25 local rapporte **0 fichiers indexés, 2061 skipped,
844 ms de scan**. Ce n'était pas un rattrapage documentaire volumineux.

GrepAI v0.37.0 : `cli/watch.go:1183` appelle `BuildFull` avant la readiness ;
`rpg/indexer.go:78–87` efface le graphe en mémoire, puis redemande features et
résumés (`139`, `179`, `224`). Les données persistées restent lisibles séparément.
Sources vérifiées : https://github.com/yoanbernabeu/grepai/tree/v0.37.0
(cli/watch.go, rpg/indexer.go, rpg/summary.go, store/gob.go, daemon/daemon.go).
Les fichiers GOB sont remplacés atomiquement, mais ne constituent pas une
transaction commune entre index vectoriel, symbolique et RPG.

## Évolution / impact
- La reprise a trouvé une correction déjà présente dans les scripts : retour
  après enregistrement du daemon, indication MCP `warming`/`ready`, lecture des
  snapshots persistés. Ces modifications ont été préservées, pas réécrites.
- Défaut résiduel corrigé : un dépôt Git, même principal, peut publier
  `grepai-worktree-<id>.pid/.ready`, et non `grepai-watch.pid/.ready`.
  Découverte bornée des marqueurs exacts du PID, contrôle d'identité OS maintenu.
- Tests : 37 PASS avant ajout du test dédié à la readiness Git ; validation de
  ce dernier et appel MCP réel à faire. Le succès runtime n'est pas encore acquis.

## Limites et suite
La correction de disponibilité ne rend pas incrémental le `BuildFull` de GrepAI.
Le cache LiteLLM peut réutiliser des requêtes identiques, sans garantir leur
équivalence ni supprimer le coût de reconstruction. Pas de patch du binaire
GrepAI, pas de suppression d'index, pas de downgrade xhigh.

Serena est attaché à la bonne racine AIForAll ; les scripts globaux sous
`C:/Users/djden/.config/1mcp/lifecycle` sont hors couverture. Références exactes
vérifiées dans ces scripts et tests par recherche ciblée.

## Recommandation / décision
Valider la disponibilité réelle puis signaler séparément le coût RPG résiduel.
Aucune nouvelle décision utilisateur requise pour ces contrôles bornés.
