# La revue préserve les garanties ; validation complète autorisée en cours de lancement

La mission doit prouver la reprise des notifications après une panne, sans perte ni accès indu. Le correctif ne change que le moment où le test vérifie la date de nouvelle tentative.

L'examen indépendant est terminé : **aucun défaut bloquant ni suggestion** dans ce correctif. La comparaison reste stricte, à l'heure réellement courante après la lecture SQL ; absence de ligne, erreur ou échéance atteinte font échouer le test. Les observations de douze secondes et la suite reconstruction/replay/autorisation restent conservées. Ce verdict est statique et borné, pas une réussite du scénario.

Le lead a persisté le résultat et son index, puis vérifié de nouveau la source exacte, le formatage avec `rustfmt --check` et le diffcheck : succès. SDKd60 propre, AI1f2, unique source modifiée de34 lignes ajoutées/1 supprimée ; aucun Cargo concurrent. Docker est actif ; inventaire frais de onze conteneurs préexistants, dont un étranger actif, aucun adopté.

La prochaine étape est **l'unique validation complète autorisée en106**, sur la source intégrée et revue, sans nouvelle décision humaine nécessaire. Le lanceur existant ne change que le nom de campagne et la sélection de la cible verticale. Il vérifie les images en cache, recapture avant les créations, inscrit les identités et assure le cleanup même sur échec. Seules les nouvelles fixtures possédées pourront être arrêtées gracieusement, avec perte de leur mémoire volatile ; aucune ressource préexistante ou volume n'est supprimé, aucun arrêt global Docker/WSL.

Pas de NoOp redondant, nouveau harness, retry jusqu'au vert, correction de production ou publication. En cas d'échec, conserver les faits et rendre le blocage. Les échecs97/105 restent immuables quelle que soit l'issue.

## Preuves techniques

- Revue indépendante persistée : `.cursor/review-briefings/20261027T0841Z-tests-temporal-oracle106-1f2dd30.md` et INDEX à la racine principale.
- Source intégrée SHA256 `6ac7be0e9e220a501241a0b48bf261c6fd7e14aa921ab098dbbfa29e7df071e1`, patch `ceb2b730fea2f8524b56243dd263daacc8441a0204b3d6e12b1362db8bd5dad0`.
- Lanceur TI/.tmp/vertical-temporal-validation106-inputs/run-validation106.cjs, SHA256 `d88da02353b14a96e271d38e2aa4ced61c8db5bf0deecf2c550cd40b051249dd`, syntaxe0 et comparaison avec lanceur existant contrôlées.
- Commande exacte : `cargo test --locked -p telegraph-service --test notification_created_routing_test user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery -- --exact --test-threads=1`, Windows natif,24jobs configurés,targetE:.
- Bail lead : une seule admission marquée avant lancement, libération à sortie Cargo puis cleanup ; pas d'autre consommateur autorisé. Les fixtures précédentes SMTP Exited2/volumesASK restent hors bail.
