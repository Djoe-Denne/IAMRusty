# Consommateur intégré ; vérifier le canal avant toute activation

## Résultat intégré

Le code de lecture privée proposé est intégré dans quatre fichiers, après
inspection et comparaison exacte avec le candidat. Le lead a réexécuté les
**162 tests Windows avec succès** ; huit tests POSIX ne sont pas exécutés sur
Windows et devront être vérifiés sous Linux. Les garanties natives ne sont donc
pas encore prouvées pour ce nouveau code.

Le canal nécessite à la fois l'option explicite et une session parent en mémoire.
Le parent doit réellement vérifier l'hôte, le bail, les preuves du refus local et
le rendez-vous de lecture ; une fixture ou un manifeste ne le remplace pas.
Les sorties actuelles ne reçoivent aucun champ de site ou chemin de relevé.

## Suite autorisée

Une revue indépendante fresh examinera confidentialité, permissions, activation
et refus. En parallèle, le lead peut exécuter les tests avec des données fictives
connues sous Linux, sur une copie propre du même code. Ces travaux n'ont aucune
dépendance mutuelle et ne produisent pas de relevé Chromium réel.

Les corrections éventuelles attendront le rapport de revue persisté. Aucune
capture ni relance navigateur n'est admise avant la fin de ces vérifications
et l'organisation de la lecture locale avec l'utilisateur. Chromium, HTTP et la
cause du refus restent ouverts ; un succès des tests ne les rendra pas verts.

## Preuves techniques et limites

- Source intégrée `968592f2efec114d6f7c0c1c52c84fb07b505cab`, base1c0036f.
- Quatre fichiers byte-identiques au candidat ;18autres fichiers de base inchangés,
  dont superviseur, CI, pins, scénarios et provisionneur. Hook normal et diff-check0.
- Lead Windows :162/0.945s/exit0/skips8,154tests exercés. ROUGE initial10/10échecs
  attribué à l'auteur, non rejoué par le lead ; aucune extrapolation Linux.
- Sauvegarde22fichiers source/workflow et index vérifiée :
  `.tmp/k6-private-site-source-968592f-b1fae048ceeb4baf9050775c6328909b`.
- Briefing indépendant préparé :
  `.cursor/review-briefings/20261030-k6-private-site-security.md`.
- Aucun site réel collecté, runtime AIForAll arrêté à cette photographie,
  outils/disque conservés. Aucun nouveau déploiement, compte, secret ou sandbox modifié.
