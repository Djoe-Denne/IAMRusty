# Parcours qualifié et tests complets ; arrêt du laboratoire possédé

La revue indépendante juge les oracles du navigateur recevables, sans défaut
bloquant supplémentaire. Les trois suites contractuelles passent maintenant :
**17 tests d'intégration, zéro échec, test ignoré ou filtré**. Les trois fixtures
créées sont retirées et les ressources préexistantes sont préservées.

Le parcours navigateur et les refus d'accès sont qualifiés pour ce laboratoire,
pas pour la production. Les limites restent explicites : certains compteurs
sont absents, les refus avant IAM sont inférés par conservation des requêtes et
les effets des routes ; aucune mesure directe de base de données ni capture
complète de configuration active n'est revendiquée.

Les vérifications restantes portent uniquement sur des pièces figées et la
documentation. Je libère donc les trois nœuds Kind créés pour cette mission par
arrêt gracieux. Leur mémoire volatile sera perdue ; conteneurs arrêtés, volumes,
images et cluster ne seront pas supprimés. Le conteneur étranger actif reste
intact ; aucun arrêt global de Docker Desktop ou WSL.

## Preuves et périmètre de l'arrêt

- Source exécutable7b8c1f95/SDK02e5 ;8606173 ne change que sélection QA/docs,
  comparaison des arbres applicatifs/Cargo/SDK vide. Image et navigateur restent
  liés au code intégré, sans reconstruction inutile pour les seuls scripts QA.
- Tests complémentaires :6PASS/exit0/51.343s et2PASS/exit0/33.048s ; journaux
  `.tmp/oauth-remaining-it-b57e446a5d5544cc927fa7cded96517f/`.
- Registres lus avant cleanup :2634lignes,193 anciennes lignes malformées sans
  ambiguïté sur les trois IDs ; aucun bail étranger trouvé pour ces nœuds.
- IDs exacts sous bail6b7 :
  -024afb39bcfc4bc4e59a4b518d23d084fe70a80220242aff6e23b9c76539f903
  -6fd561584da07159621e52e04580dacbbc0dfabdf7a79a221e730ea2da89b7f1
  -0db7385cafb423fcc1bfee6c4f56dc9588d133fc988d676e7b495becdb0faf6d
- Arrêt sans timeout de mise à mort, vérification des états et événements append-only
  au registre29 ; résultat réel à consigner après opération.

Résultat réel : arrêt réussi en275.185s, trois nœuds nonrunning/Exited130,
aucune suppression. SixIDs conservés ; les trois autres conteneurs et leurs états
restent inchangés. Les volumes/images sont conservés et DockerDesktop/WSL ne sont
pas arrêtés globalement. Aucun conteneur de cette campagne ne reste actif.
Infra final : PASS documentaire borné, pas qualification globale de toute la machine.
