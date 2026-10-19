# Archives ops — non nominales

Ces sources historiques sont conservées, pas supprimées ni déclarées inutiles.
Elles ne constituent ni le [parcours runtime actif](../../../ops/deploy/README.md),
ni une preuve de succès live. Les ADR historiques restent inchangées ; cette
table résout leurs anciens chemins sans réécrire leurs décisions.

- Ancien `ops/deploy/PACKAGE-D-CHECKPOINT.md` (cité par ADR 0603 et 0606) :
  [checkpoint historique](PACKAGE-D-CHECKPOINT.md).
- Ancien `ops/scripts/layout-rewrite/` :
  [toolkit historique et ses 21 sources/fixtures](layout-rewrite/README.md).
  La structure interne et les assertions du self-check sont conservées.
  Self-check non hermétique par conception : inputs simulés privés N2 non
  transférés, aucune clé. Pré-déplacement OK dans l'arbre canonique (constat du
  lead) ; post-déplacement sur checkout propre : trois assertions rouges
  **conservées**, `prefix ./certs/x`, `prefix docker-build-stage`,
  `compose volume certs` (`Rewrite-Layout.ps1:1211/1212/1219`). Aucune assertion
  retirée ; voir la [limite documentée](layout-rewrite/README.md#limite-historique-du-self-check-sur-checkout-propre).
- Ancien `ops/scripts/rename-crates.sh` : [renamer historique](rename-crates.sh).
- Ancien `ops/deploy/apps/overlays/kind-demo-monolith/reference-kv-plugin-service.yaml` :
  [Service historique à URL manuelle](kind-demo-monolith/reference-kv-plugin-service.yaml).
- Ancien `ops/deploy/apps/overlays/kind-demo-monolith/seed-reference-kv-grants.sql` :
  [seed SQL historique](kind-demo-monolith/seed-reference-kv-grants.sql), pas un
  chemin d'acceptation métier.

`enroll-job.yaml` **reste actif à son chemin legacy** :
[Job enroll](../../../ops/deploy/apps/overlays/kind-demo-monolith/enroll-job.yaml).
`schedule-reference-kv.ps1` l'applique encore ; il n'est donc pas archivé.
Les capacités des recettes legacy ne sont pas redirigées vers local-full.

Le driver historique 0605 reste accessible par `just prove-gold`, désormais à
[ops/tests/legacy/prove-gold-path.ps1](../../../ops/tests/legacy/prove-gold-path.ps1).
Il conserve ses mutations legacy et n'est pas le nominal full.

## Limites de cette migration

- Seuls les fichiers suivis du toolkit ont été déplacés. Les copies privées
  ignorées observées sous l'ancien `ops/scripts/layout-rewrite/fixtures/`
  restent en place (contenu/propriété inconnus, aucun nettoyage autorisé).
  Le [self-check historique](layout-rewrite/README.md#limite-historique-du-self-check-sur-checkout-propre)
  reste rouge sur checkout propre (trois assertions dépendantes de ces copies) ;
  aucune assertion ni donnée privée n'est modifiée pour fabriquer un PASS.
- `ops/scripts/start-all.bat` et `ops/scripts/start-all.sh` restent
  **DEFER-UNKNOWN** : aucun changement de comportement, aucun lancement effectué.
  L'absence de consommateur automatisé identifié ne prouve pas l'absence d'usage
  humain ; réouverture si un consommateur est trouvé ou sur arbitrage utilisateur.
  Lecture seule : ces lanceurs dépendent du `docker-compose.yml` du répertoire
  courant et du binaire legacy `docker-compose` ; ils exposent aussi des actions
  destructives (`down -v`, `docker system prune -f`). Leur conservation ne vaut
  ni autorisation de les exécuter ni validation de leurs comportements.
- Les anciens rapports suivis `.tmp-layout-rewrite/` sont des captures
  historiques immuables, pas des commandes à réexécuter ni un état courant.
