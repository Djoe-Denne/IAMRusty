# Corriger les ancêtres Windows et l'alerte de rétention

## Objectif et problèmes concrets

Le conducteur doit garantir le lancement de l'instance attendue et signaler si
le relevé privé ne peut pas être retiré. La revue indépendante relève deux défauts
bornés, à résoudre avant le préflight réel puis la collecte.

Le contrôle Windows vérifie le dossier et le disque finaux, mais pas tous leurs
ancêtres. Si un autre compte pouvait renommer ou rediriger un ancêtre, une autre
image pourrait commencer à s'ouvrir avant le refus post-lancement. **Cette capacité
n'a pas été observée sur l'installation actuelle** : ce n'est pas un constat de
compromission de l'hôte.

Le second défaut est reproduit avec des données entièrement fictives : si le
retrait du relevé échoue, le module garde correctement l'état « rétention non
prouvée », mais le conducteur le transforme en refus générique. Le résultat reste
non nul ; aucun faux PASS n'est démontré. L'opérateur ne reçoit toutefois pas
l'avertissement spécifique dont il a besoin.

## Décision et suite

Le lead exige les deux corrections dans le conducteur, avec tests de régression
avant correction lorsque possible. Les contrôles des ancêtres doivent précéder
tout lancement et être renouvelés ; une chaîne indéterminée sera refusée, sans
modification automatique des ACL, les règles d'accès Windows, des parents existants.

L'échec de retrait devra traverser le protocole sous un diagnostic fermé explicite,
sans chemin, contenu ou exception libre, en conservant le non-zéro et l'absence
de clôture normale. Le consommateur déjà testé, les pins, scénarios et CI restent
inchangés. Une fermeture de revue sera bornée à ces correctifs et à leurs tests.

**Aucun préflight réel, lancement Chromium ou relevé réel n'est engagé.** L'auteur
reprend dans la même session après intégration du rapport et du brief. Il n'y a
pas de nouvelle demande de GO humain pour ces correctifs de contrat.

## Preuves et limites

- Revue fresh READONLY : deux MEDIUM, aucun CRITICAL/HIGH, source `195059a`.
- Rapport complet : `20261030T1428Z-security-k6-private-parent-4560b18.md`.
- Reviewer : cinq tests ciblés réussis et contre-exemple synthétique de rétention
  confirmé. L'hypothèse de DACL nulle a été testée et réfutée, pas retenue.
- Tests antérieurs Windows189/skips8 et Linux189/skips11 restent valides à leur
  portée ; ils ne couvraient pas ces défauts. AIForAll est arrêté, bail libéré,
  sauvegardes conservées et aucun site réel créé. Mission toujours ouverte.
