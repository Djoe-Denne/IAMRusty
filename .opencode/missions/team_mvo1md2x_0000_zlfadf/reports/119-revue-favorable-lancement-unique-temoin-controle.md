# Revue favorable, lancement unique du témoin contrôlé

La revue indépendante conclut à un **PASS borné de validité du test**, sans défaut bloquant. Elle vérifie le claim SQL réellement terminé, l'annulation locale et la jointure des propriétaires, la fenêtre de 25 secondes contenue dans le verrou mesuré de 30 secondes, les vraies sélections du dispatcher et les protections contre les cycles partiels ou l'inactivité. Aucun test n'a été exécuté par le reviewer.

La portée reste diagnostique : ce témoin n'exerce pas l'application entière, ne prouve pas l'origine historique de109 et n'observe pas la publication après expiration. Une lecture SQS vide restera formulée comme « aucun message observé », pas comme une garantie universelle d'absence.

Le lead a persisté la revue et son index, terminé sa tâche native, puis vérifié à nouveau la source exacte, `rustfmt --check` et le diffcheck : succès. AI1f2, SDKd60 propre, aucun Cargo concurrent ; 24 jobs configurés, target E:, aucun réglage contradictoire. Inventaire frais : treize conteneurs préexistants, un seul étranger actif, aucun adopté.

Le lead lance maintenant **l'unique exécution117 déjà autorisée**, via le lanceur existant limité au nouveau filtre. Celui-ci recapture l'inventaire, vérifie les images présentes, trace les nouvelles identités et termine leur cleanup même en cas d'échec. Les seules nouvelles fixtures possédées pourront être arrêtées gracieusement, avec perte de leur mémoire volatile ; les ressources préexistantes et volumesASK restent préservés, sans arrêt global Docker/WSL.

Pas de relance automatique, verticale supplémentaire, modification de production ou délai augmenté. Un résultat conforme au pronostic du composant ne ferme pas109 et n'autorise pas publication/pin ou acceptation globale.

## Annexe

- Revue : `.cursor/review-briefings/20261027-tests-controlled-claim117.md`, INDEX principal ; tâche `task_mvqm2ajz_0000_ey1lby` terminée.
- Source intégrée SHA256 `403e8c90c1b59473c161982bd981a34749797ef09e523423abad92f90124160b`, patch `99ecdf11e7576673e1386da344786e286b37bdd8eb5becefc2bdc17ab4c45c87` inchangés depuis revue/compilation.
- Lanceur TI/.tmp/controlled-claim117-run-inputs/run-component117.cjs, SHA256 `dcbeb083ed16f8b8f43901edeed9d59c253a27e2752e42be4b7f84461fc32c7e`, syntaxe0. Seuls préfixe, cible unique et filtre diffèrent du lanceur historique inspecté.
- Commande : `cargo test --locked -p telegraph-service --test outbox_runtime_tests controlled_committed_claim_cancellation_keeps_default_lock_through_25s_observation -- --exact --test-threads=1`.
- Bail lead pour un seul Cargo et ses fixtures, admission marquée avant exécution, libération à sortie puis cleanup vérifié. Aucun autre consommateur actif de ce bail.
