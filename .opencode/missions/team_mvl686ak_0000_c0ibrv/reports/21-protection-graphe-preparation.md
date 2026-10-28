# 21 — Protection du graphe : préparation sans interrompre AIForAll

## Objectif / étape
Préserver les index persistés et le futur résultat de l'enrichissement actif.
Exécution directe, sans délégation. Aucun redémarrage du watcher, du manager ou
de LiteLLM admis pour appliquer cette protection.

## Éléments établis
- Continuité : rapports 17–20 et ledger
  `.cursor/handoffs/20261022-mcp-optimization.md`.
- À 19:59Z, le job désigne toujours watcher 51668, génération
  `2026-10-23T17:22:12.5511750Z`, moniteur 30312 ; phase persistée `warming`.
- Code source natif déjà examiné : BuildFull remet le graphe à zéro ; les
  lectures RPG qui ferment le store peuvent persister une copie ancienne.
- La première ébauche ajoute des copies versionnées et un heartbeat sans lecture
  RPG, mais **n'est pas activée**. Son erreur de sauvegarde pourrait actuellement
  atteindre le chemin d'arrêt de campagne : défaut bloquant avant activation.
- Base Git observée : `93e376c5530a439aaafe61a64d3e881865c1acf1`. Nombreux travaux
  utilisateur indexés/non indexés : intacts, aucun commit/stash/reset prévu.

## Choix et vérifications bornées
1. Isoler les erreurs de sauvegarde : aucune erreur de copie ou de journalisation
   ne doit déclencher l'arrêt du worker.
2. Tester conservation des sources, exclusion des credentials, versions distinctes,
   fichiers manquants/erreurs et copie instable.
3. Créer une sauvegarde immédiate des **seuls fichiers sur disque** ; vérifier
   SHA256 et lecture native sur une copie jetable, jamais sur le canon.
4. Remplacer uniquement le moniteur identifié, reprendre le même job et vérifier
   son heartbeat `grepai_list_workspaces`. Conserver les offsets et compteurs.
5. Programmer des snapshots avant audit final et après arrêt gracieux normal.

## Limites / recommandation
Les sauvegardes ne contiennent pas les progrès encore uniquement en RAM. La
lecture seule des copies protège contre l'écrasement ordinaire, pas contre toute
suppression volontaire ou panne du disque. Aucun ancien snapshot supprimé.
La réutilisation incrémentale native reste un chantier distinct : ne pas prétendre
que les copies seules empêchent les recalculs complets futurs.

Serena est bien attaché à AIForAll ; les scripts visés vivent dans LocalAppData,
hors de sa couverture. Inspection ciblée des scripts et de leurs usages exacts,
sans reconfiguration du serveur partagé. Aucun Docker/Kind/WSL/Cargo requis.
