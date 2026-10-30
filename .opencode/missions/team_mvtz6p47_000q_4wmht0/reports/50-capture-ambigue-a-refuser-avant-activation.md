# Refuser une capture ambiguë avant d'activer le canal privé

## Problème et conséquence

La revue indépendante a trouvé un cas non couvert : un FATAL complet accompagné
d'un second FATAL tronqué pourrait encore produire un relevé privé. Le code ne
compte actuellement que les messages complets. Or le contrat exige de ne rien
relever si la capture est ambiguë ou incomplète.

Le risque est limité au fichier local d'une session déjà autorisée ; aucune fuite
publique ou élévation de privilèges n'a été démontrée. Le contre-exemple est établi
par lecture du code, pas par collecte réelle. Les162tests Linux réussis ne
couvraient pas cette combinaison précise.

## Décision et suite

Le lead retient la correction avant activation. L'auteur ajoutera d'abord le test
de régression, puis un veto conservateur dans le seul consommateur privé. Il ne
changera ni les formats positifs acceptés, le classificateur public, les arguments
Chromium, les pins ou les verdicts fonctionnels. Aucun nouvel essai réel n'est utile.

Le rapport complet est persisté avant cette reprise. La correction sera contrôlée
au diff et par les tests ciblés puis complets. Une relecture de fermeture sera
limitée à ce refus et à ses tests, sans nouveau réaudit général.

La collecte reste désactivée. Même après correction, le vrai contrôle parent et
le créneau de lecture locale restent à organiser. Il ne s'agit pas d'un nouveau
choix d'hôte ni d'un GO général à redemander.

## Preuves et limites

- Revue fresh READONLY : un MEDIUM, aucun CRITICAL/HIGH ; source968592f.
- Rapport complet : `.cursor/review-briefings/20261030T1243Z-security-k6-private-site-eb3d355.md`.
- Index de mission actualisé, pas d'ajout forcé à l'index local général.
- Le reviewer a vérifié le diff, pas lancé unittest ou un runtime. Les162tests
  Linux parallèles sont une preuve séparée, pas une validation du cas manquant.
- AIForAll Stopped, disque/outils conservés, aucun site réel collecté ; mission ouverte.
