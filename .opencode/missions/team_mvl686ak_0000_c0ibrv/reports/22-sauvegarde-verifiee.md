# 22 — Première sauvegarde vérifiée ; remplacement du moniteur préparé

## Résultats depuis le rapport 21
- **11 tests PASS** : versions distinctes, sources et permissions préservées,
  SHA256, absence de credentials, graphe manquant, index optionnels, modification
  concurrente, échec de copie et échec d'enregistrement de métadonnées.
- Les erreurs de sauvegarde/journalisation sont désormais isolées du handler
  d'arrêt de campagne. Les copies instables sont signalées, pas validées.
- Snapshot créé à **20:02:12Z**, trois fichiers, environ 122 Mo :
  `rpg.gob`, `index.gob`, `symbols.gob`. SHA256/taille et attribut lecture seule
  des trois copies vérifiés. Les sources ne sont pas passées en lecture seule.
- Lecture du graphe réussie avec le binaire natif sur des copies temporaires de
  cette sauvegarde. **Ce contrôle n'est pas un audit complet de couverture** :
  une seule zone découverte dans le répertoire de restauration vide ; aucune
  conclusion d'enrichissement complet ou de progression actuelle n'en découle.
- Préflight réel du heartbeat `grepai_list_workspaces` réussi à 20:03:12Z,
  avec identité attendue du watcher ; génération 51668 inchangée, phase `warming`.

## Preuves
`C:/Users/djden/AppData/Local/LiteLLM/graph-protection-tests-20261023.json`

Dans `.../rpg-enrichment-20261023-consolefix/aiforall/` :
`graph-protection-before.json`, `graph-protection-verification.json`,
`graph-protection-heartbeat-preflight.json`.

Snapshot :
`C:/Users/djden/AppData/Local/LiteLLM/graph-backups/aiforall/2026-10-23T20-02-12-687Z-b5a1eab6-5f3a-4b84-a80d-33dab17746cf/`.

## Étape suivante et limites
Remplacer **seulement** le moniteur 30312 après contrôle date/commande/lock.
Reprendre `--follow` sur le même dossier, sans `--initialize`, sans stopper le
watcher 51668, le manager 5380 ou LiteLLM. Archiver son lock et son job courant,
préserver offsets, compteurs et baseline. Vérifier immédiatement la continuité.

Les sauvegardes avant audit final et après arrêt gracieux normal sont codées,
mais leur exécution future ne sera prouvée qu'à la fin de campagne. Le snapshot
actuel ne capture pas le travail en RAM. Aucune restauration sur le canon actif,
aucune suppression d'anciens backups, aucun nouveau calcul lancé.
