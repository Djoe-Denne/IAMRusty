# 44 — Clôture des trois sessions, métadonnées seulement

## Résultat et stade
Les3sessions demandées sont terminées, CLIexit0/outcome succeeded, contrôleurs
finished-awaiting-audit sans erreur de pagination. Audit des traces terminé.
Pour ne pas recréer d'aide à la réponse métier dans le checkout, le bilan détaillé
est conservé **hors dépôt** et remis par lien dans la réponse utilisateur.
Ce document n'en reproduit pas les conclusions techniques ni les sources d'oracle.

## Intégrité vérifiée
- Modèles/variantes conformes dans tous les messages assistant : Luna high,
  Luna xhigh, Haiku5.5 via OpenCodeGo medium ; aucun fallback de modèle.
- Un seul prompt chacune, même hash livré
  baf6de30c9d9e3ff0e36c826dcec1bd72e9a048015b3ef63b3dddf695d34e827.
  Corps inchangé, mêmes guillemets de transport CLI qu'aux précédents essais.
-24fichiers suivis présents, mêmes empreintes aux3départs et après fin ;
  snapshots sans changement de fichier. Pas d'écriture de résultat pendant runs.
- Les12anciens fichiers sont restés absents ; copies archivées au hash initial,
  aucune lecture d'ancien audit ou mention de son nom de fichier observée dans
  les sorties examinées. Pas une preuve d'isolement absolu des index/caches.
- Deuxpages parsession dont dernièrevide. Captures réutilisées pour l'audit,
  pas de relance des tâches ou d'API pour rééchantillonner un résultat.

## Mesures descriptives
- Luna high :65,498s,31appelsdirects,13messagesdont10assistant.
- Luna xhigh :387,743s,55appelsdirects,23messagesdont20assistant.
- Haiku medium :66,020s,13appelsdirects,10messagesdont7assistant.
Concurrence, profondeurs et méthodes différentes : pas de benchmark isolé ou
gain financier démontré. Retour sur investissement de la bibliothèque non établi.

## Périmètre et clôture
Aucun changement de consigne ou bibliothèque pour forcer le résultat observé,
aucune session supplémentaire. Les anciens fichiers restent archivés, pas remis
en place ; le bilan détaillé ne devient pas un nouvel oracle dans ce checkout.
Pas d'édition applicative, tests/buildapplicatifs, Cargo, Docker/Kind/WSL, ressource
Docker, purge/restartMCP ou déchargement8B. Les3CLI sont terminés, aucun bail
supplémentaire ou cleanup infra nécessaire. Aucun commit/stash/reset/merge/push.
