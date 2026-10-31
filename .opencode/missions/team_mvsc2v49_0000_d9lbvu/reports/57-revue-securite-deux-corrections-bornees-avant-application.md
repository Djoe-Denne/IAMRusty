# Revue sécurité : deux corrections bornées avant application

L'objectif reste de rétablir l'entrée du navigateur sans élargir les permissions.
La revue indépendante ne démontre aucun défaut critique ou élevé, mais elle a
trouvé deux problèmes à corriger avant toute application réseau.

Le premier aurait refusé les observations légitimes d'Envoy : le garde natif et
les documents JSON représentent différemment la même liste de propriétaires.
Le cas a été reproduit, puis corrigé sans relâcher les contrôles d'identité.
Les six tests ciblés passent maintenant, y compris les refus d'identités modifiées.

Le second concerne une modification concurrente de la règle réseau. Le code
prend sa version dans une lecture effectuée après la modification. Un autre
écrivain pourrait intervenir entre les deux ; un retrait ultérieur risquerait
alors de toucher sa règle. Il faut retenir la version renvoyée par la modification
elle-même. L'architecte prépare l'extension strictement nécessaire du garde :
réponse JSON pour cette seule règle, sans autoriser de lecture de secrets ou de
nouveau transport.

Aucune règle réseau n'a été modifiée. Je poursuis ces corrections dans le mandat
déjà délégué, avec tests ciblés et relecture indépendante du delta final. Le
diagnostic réseau établi n'est pas recommencé et aucune refonte n'est engagée.

## Preuves et limites

- Revue : `.cursor/review-briefings/20261029T1233Z-security-envoy-ipip-ingress-ff12007.md`.
- Identité native : nouveau test rouge avant correction, puis suite6PASS,
  aucun skip, exit0, 2.345s ; changement d'une comparaison canonique seulement.
- Course concurrente : constat statique désormais reproduit par le nouvel oracle
  nommé avant exécution ; un test rouge confirme l'adoption indue. Sa correction
  reste à effectuer ; ce n'est pas un échec d'environnement.
- Les anciens tests simulaient uniquement la représentation JSON et une
  concurrence plus tardive : ces deux fenêtres manquaient à leurs oracles.
- Tests simulés et revue ne qualifient ni trajet réel, navigateur ou production.
