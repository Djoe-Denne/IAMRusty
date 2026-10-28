# Observer la cause de l'échec sans changer les garanties

## Objectif et étape

La notification doit survivre à une panne de messagerie puis devenir accessible à son destinataire. Le dernier test complet échoue sur la date de nouvelle tentative. L'utilisateur a autorisé un seul diagnostic pour distinguer une date incorrecte d'une vérification trop tardive, sans correction automatique.

La préparation est maintenant inspectée par le lead : compilation sans exécution et formatage réussis. L'instrumentation ne modifie que le test ; les dates seront observées après l'enregistrement puis au moment de la comparaison. L'assertion exige toujours une date postérieure à l'heure courante. L'attente d'observation et le scénario de reprise restent inchangés. Aucun test diagnostique encore exécuté au présent checkpoint.

## Vérifications et limites

- Base et SDK vérifiés dans l'arbre isolé ; SDK propre. Unique modification de source : test vertical, +93/−1. Sampler et suffixe reprise/replay/autorisation comparés exactement aux sources de référence. Une première comparaison du sampler incluait les nouveaux helpers et retournait faux ; la comparaison bornée à son corps donne égalité exacte.
- La configuration réellement passée au dispatcher utilise bien les valeurs par défaut : poll 2 secondes, délai de base 5 secondes, 20 tentatives maximales. La limite absolue SQL de 25 secondes est conservée. Aucun changement de production.
- Archive source/patch/manifest et preuves de compilation copiées avec empreintes identiques dans le dossier technique ci-dessous.
- Docker est déjà actif. Inventaire frais : dix conteneurs préexistants, dont neuf arrêtés et un étranger actif ; aucun n'est adopté. Aucun Cargo observé. Configuration effective : 24 jobs internes, target E:.
- Lanceur existant relu : une seule cible verticale, mêmes fixtures et cleanup par identités retournées ; seuls identifiant de campagne, sélection de cible et destination du registre historique sont adaptés. Syntaxe contrôlée. Pas de nouveau harness, NoOp, publication, Kind ou répétition jusqu'au vert.

## Suite autorisée

Le lead prend le slot Cargo pour une seule exécution via ce lanceur, qui recapture les inventaires avant les créations, vérifie les images déjà présentes, inscrit les identités et termine le cleanup même en cas d'échec. Seules les nouvelles fixtures possédées pourront être arrêtées gracieusement ; leur mémoire volatile sera perdue. Les ressources préexistantes et volumes restent préservés. Le runtime étranger interdit un arrêt global Docker/WSL.

Un résultat vert ne supprimera pas l'échec antérieur. Une observation manquante sera inconclusive. Toute correction éventuelle sera présentée séparément ; publication et référence finale restent bloquées.

## Annexe technique

- AI : `1f2dd30af52a707ceb4abc2fbea3f963307c246f` ; SDK : `d60a88b2f2f499768a807941a1b4c80c9a366b3f`.
- Source SHA256 : `edc39c12ca94852de79168f848dcdc46895c184215c90938f9969ef623b76dec` ; patch : `8a6bdd653896399cfd6d4a5bee99b40e9c9af464ebaf69de31a53840ca775108`.
- Lanceur : root pool14, `.tmp/vertical-diagnostic-inputs/run-vertical-diagnostic.cjs`, SHA256 `030ae4a78f993d28c7d21ff969bf14f1b46d463e8581b8781fd09b1803d25ae7`.
- Archive : `C:/Users/djden/source/repos/AIForAll/.worktrees/ti/.tmp/vertical-diagnostic-approved-20261027/`.
- Exécution prévue : `cargo test --locked -p telegraph-service --test notification_created_routing_test user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery -- --exact --test-threads=1`.
- Localisation : helper temporaire et points SQL/assertion du seul test. Impact : lectures SQL passives et traces sans payload/secret ; aucune garantie d'absence totale de perturbation temporelle. Limite : diagnostic unique, pas validation globale.
