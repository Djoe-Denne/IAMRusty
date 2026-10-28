# 17 — Seul AIForAll continue

## Décision exécutée
Utilisateur : « tu vas en arrêter deux, tu conserves que AIForAll ».
Arrêt des seuls moniteurs ARTIST4224 et retro-eng38056, puis arrêt natif gracieux
des watchers ARTIST52008 et retro-eng52064. Vérification préalable des dates de
création, commandes exactes et propriété courante ; locks des moniteurs archivés.
Les notifications shell exit1 sont la conséquence de cet arrêt demandé.

## Vérification finale
- AIForAll : **watcher51668 vivant, même génération ; moniteur30312 vivant**,
  progression `running`. Aucun arrêt ni redémarrage de ces processus.
- ARTIST : ancien watcher et moniteur absents, progression `stopped_by_user`.
- re-ff8/retro-eng : ancien watcher et moniteur absents, `stopped_by_user`.
- LiteLLM partagé conservé, endpoint liveliness **HTTP200**. Aucun restart ni
  nouveau cap local, index et caches conservés.

`user-stopped.json` et logs de stop dans chacun des deux dossiers conservent la
preuve. Les états JSON/Markdown et l'index des suivis ont été mis à jour sans
présenter ces arrêts comme des enrichissements réussis.

## Périmètre / continuité
Seules ces deux campagnes en arrière-plan sont arrêtées ; configurations,
catalogues et données des projets ne sont ni supprimés ni désactivés définitivement.
Le moniteur AIForAll et son bail restent nécessaires au travail demandé. Attendre
la notification du shell AIForAll `sh_169c916840013YMhcLJVrwHGyf`, sans polling.
Aucun Docker/Kind/WSL/Cargo, aucun commit/push, aucune suppression d'index.
