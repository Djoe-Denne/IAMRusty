# 23 — Protection active, calcul AIForAll conservé

## Résultat
Deux versions des trois index persistés sauvegardées sous
`C:/Users/djden/AppData/Local/LiteLLM/graph-backups/aiforall/` :
- `2026-10-23T20-02-12-687Z-b5a1eab6-5f3a-4b84-a80d-33dab17746cf` ;
- `2026-10-23T20-04-09-044Z-b06fe8cc-1f94-4bbf-bd1e-83beb5ef408e`.

Les six copies ont des SHA256 revérifiés et sont en lecture seule ; manifeste
par version, journal append-only, aucune purge automatique. Configurations,
credentials et cache LiteLLM exclus. Lecture native réussie sur une copie isolée
du premier graphe, sans prétendre à un audit complet de couverture.

## Activation vérifiée à 20:06:19Z
- **Watcher 51668 inchangé**, même génération depuis 17:22:12Z.
- Manager 5380 inchangé ; proxy Python 26428 toujours de génération 16:40:37Z.
  Aucun restart ni flush du cache entrepris.
- Seul le moniteur 30312 a été remplacé par **14780**, génération 20:04:07Z,
  `--follow` sur le même job. Départ, baseline, compteurs et offsets conservés.
- Heartbeat devenu `grepai_list_workspaces`, ancien `grepai_index_status` tracé.
  Après 132 s de fonctionnement, le graphe canonique conserve sa date 20:03:18Z :
  aucune réécriture observée depuis le nouveau moniteur. Le canon reste modifiable
  par le vrai writer ; statut du travail toujours `running`.
- ARTIST et retro-eng : anciens watchers et moniteurs toujours absents.

## Vérification / preuves
11 tests PASS : protection des sources, versionnement, intégrité, secret exclu,
erreurs de copie/journalisation isolées, copie instable non validée ; régressions
du suivi et du calcul de couverture. Voir rapport 22 pour l'essai de récupération.

Preuves dans `.../rpg-enrichment-20261023-consolefix/aiforall/` :
`graph-protection-activation-proof.json`, `graph-protection-final-proof.json`,
plus les preuves de snapshot, de restauration et le job/lock archivés.
Tests : `.../LiteLLM/graph-protection-tests-20261023.json`.

## Limites et suite
Les deux versions sont des **données déjà persistées**, pas le travail encore en
RAM. Snapshots programmés avant audit final et après arrêt gracieux ; ces étapes
futures ne sont pas annoncées exécutées. Une erreur de sauvegarde seule ne peut
plus atteindre le handler d'arrêt de campagne.

**Le recalcul complet au prochain démarrage n'est pas encore corrigé.**
`BuildFull` natif n'a pas été modifié. La réutilisation des nœuds inchangés et les
checkpoints depuis le writer demandent une correction native distincte, cadrée
dans `C:/Users/djden/AppData/Local/LiteLLM/GRAPH-PROTECTION.md`.
Pas de reconstruction supplémentaire lancée, pas de restauration sur un writer
vivant, pas de changement de modèle ni plafond ajouté.

## Continuité / ressources
Nouveau shell à notification automatique : `sh_16a5d1461001xaTOuwzoO4C4Y0`.
L'exit1 de `sh_169c916840013YMhcLJVrwHGyf` est le remplacement volontaire du
moniteur, pas une panne du worker. Pas de polling d'attente.
Worker, moniteur et proxy restent requis par la campagne utilisateur active.
Aucun Docker/Kind/WSL/Cargo, aucune ressource Docker créée, aucun commit/push.
Travaux Git utilisateur préservés. Les copies jetables de test ont été retirées,
les deux sauvegardes réelles conservées.

Localisation : scripts LocalAppData hors couverture Serena, examinés précisément
avec leurs usages ; racine Serena AIForAll vérifiée et non reconfigurée.
