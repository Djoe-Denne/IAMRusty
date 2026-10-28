# 18 — Estimation utile des appels AIForAll, sans interruption

## Résultat au relevé 2026-10-23T17:52:05Z
**Base prévisionnelle : environ15655 appels au proxy pour AIForAll.**
**771 réponses observées attribuables à sa connexion courante : 723 HTTP200,
48 erreurs**, soit environ **4,9 %** du volume prévisionnel d'appels.
Ces nombres donnent une estimation exploitable, pas un compteur natif exact de
nœuds enrichis. L'affirmation antérieure « aucun pourcentage possible » était
trop catégorique : un pourcentage **estimatif d'appels** est possible.

## Calcul du nombre prévu
Snapshot stable d'avant la reprise, inventorié sur copie :
- Fichiers F =2060.
- Symboles S =10230.
- Hiérarchie H =18area+107category+1180subcategory=1305.

BuildFull v0.37 appelle une extraction par fichier, une extraction par symbole,
un résumé par fichier, puis les résumés hiérarchiques :
**F+S+F+H = 15655**.
Les appels de récupération éventuels (ExtractFeature si résultat inconnu), les
changements de fichiers et l'évolution de la hiérarchie peuvent changer ce total.
Ce n'est ni un quota imposé ni une borne d'usage garantie.

## Attribution des appels observés
Lecture OS des connexions TCP établies vers4000 : PID51668 (watcherAIForAll) est
propriétaire du port source **54949**, connexion créée vers17:22:20Z, au début de
la phase RPG après le scan. Le manager5380 a une autre connexion de contrôle.
Les lignes HTTP de cette connexion sont filtrées dans la fenêtre de campagne,
reconstituée à partir des offsets et compteurs déjà conservés par le moniteur.
Pas de restart du proxy dans cette fenêtre ; première ligne de cette connexion
en position5 de la fenêtre. Les compteurs des trois anciens consommateurs ne sont
donc pas confondus pour cette estimation.

## Limites
- Comptage des réponses journalisées, pas de la requête encore en vol.
- Historique sans identifiant de projet et sans timestamps par ligne ; anciennes
  connexions fermées et réutilisation éventuelle de ports non certifiées. Ne pas
  annoncer771 comme totalité historique exacte garantie d'AIForAll.
- HTTP200 peut être un hit de cache ; ce n'est pas nécessairement un nouvel appel
  au fournisseur ni une feature valide. Les erreurs peuvent entraîner un repli.
- ~5 % des appels ne signifie pas ~5 % de résumés validés, ni ~5 % du temps total.
  Pas d'ETA fiable ou de consommation réelle de tokens déduite de ce ratio.

## Périmètre de l'intervention
Uniquement lectures job/progress/logs/connexions et sites d'appels du source local
déjà vérifié. Aucun arrêt, restart, nouveau calcul, appel génératif ou changement
de configuration. AIForAll et son moniteur continuent avec les mêmes identités.
