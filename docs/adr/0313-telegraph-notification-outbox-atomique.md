# ADR-0313 : Telegraph enregistre NotificationCreated dans la transaction de création de notification

- Statut : Proposed
- Réalité : Implemented (périmètre Telegraph ; réconciliation 2026-10-28)
- Date : 2026-10-25
- Décideurs : à renseigner après ratification humaine du texte
- Jalon concerné : fiabilité Telegraph, hors P-Apparatus
- SuperSède : aucune
- SuperSédée par : —

Accepted ratifie une cible ; Réalité décrit séparément le code.
Le statut reste Proposed : la réconciliation factuelle demandée ne ratifie pas
implicitement cette ADR. Livraison intégrée ne signifie pas quitus qualité global.

## Contexte

L'ADR [0301](0301-outbox-transactionnel-rustycog.md), Accepted / Partial,
photographiait Telegraph sans outbox et laissait son introduction ouverte. Sur la
base historique93e376c, les deux créations de NotificationServiceImpl persistaient puis appelaient
publish_notification_created : une panne transformait un succès métier en erreur
sans intention durable. Le writer rend déjà notification/livraison atomiques,
mais pas l'événement. NotificationCreated projette le droit recipient via
sentinel-sync (0303).

## Décision proposée

La création Telegraph enregistre notification, livraison éventuelle et exactement
une intention NotificationCreated dans la même transaction PostgreSQL writer.
Le port NotificationWriteRepository existant porte cette garantie, sans SeaORM
dans le domaine. IDs, données requises, payload et résultats fallibles sont
validés avant commit ; aucune publication directe post-commit ne subsiste sur
ces deux chemins. Réutiliser OutboxRecorder, migration et OutboxDispatcher partagés.

Le dispatcher ne publie qu'avec un transport réel identifié par setup. Disabled
ou dégradation NoOp suspendent les claims et conservent les événements non publiés ;
un Ok NoOp ne justifie jamais published. HTTP reste disponible. La reprise
réutilise l'événement stocké et son event_id, sans recréer les lignes métier.

Consumer et dispatcher sont supervisés dans les deux runtimes. L'arbitrage
utilisateur du 2026-10-25 autorise un sous-lot préalable strictement borné au
lifecycle cancellation-safe SqsEventConsumer, ses tests et l'intégration de son
pin. AC7 est conservé ; l'ownership des pollers et leur destruction effective
doivent être prouvés. Le [contrat d'implémentation](../contracts/telegraph-notification-outbox.md)
définit interfaces, migration, périmètre exact, budget QA proposé et oracles
AC1–AC7. Aucun Accept autonome ne découle de cette autorisation de périmètre.

## Conséquences

Une erreur de livraison/outbox avant commit annule toute la tentative. Une panne
de publication après commit ne rend plus la création erronée. Les événements
s'accumulent en mode suspendu ; les retries ont le budget fini du framework.
Les doublons de publication restent possibles : aval convergent, pas exactly-once
ni déduplication IAM entrante. Une instance de dispatcher par DB Telegraph est
la borne de cette tranche, sans garantie de fencing multi-worker. Migration
additionnelle avant le nouveau binaire, sans backfill historique.

## Alternatives écartées

- Publish-after-commit : fenêtre de perte et faux échec métier conservés.
- Transaction outbox distincte : ne rend pas notification/livraison/événement atomiques.
- NoOp traité comme livraison : published mensonger et droit jamais projeté.
- Writer OpenFGA inline ou nouveau service : change les frontières 0300/0303 sans nécessité.

## Non décidé ici

Alignement transactionnel IAM, rétention/backfill/rejeu opérateur,
scaling/fencing multi-pollers, nouveaux transports. Aucune terminaison garantie
de code synchrone non coopératif ni d'effets réseau distants. Budget QA proposé
6 s soumis à validation du contrat, pas SLA production.

## Références et réalité

- services/Telegraph/domain/src/service/notification_service.rs : deux créations déléguées au writer atomique, publication directe post-persistance supprimée.
- services/Telegraph/infra/src/repository/notification_write.rs ; services/Telegraph/domain/src/port/repository.rs : notification, livraison éventuelle et intention outbox dans une transaction writer.
- services/Telegraph/setup/src/app.rs ; runtime/monolith/src/runtime.rs : dispatcher supervisé, transport disabled/degraded suspendu, HTTP possédé selon [0503](0503-http-runtime-owned-shutdown.md).
- rustycog/rustycog-events/src/sqs.rs : défaut historique d'ownership au pin6c63b65 corrigé dans la série SDK livrée ; aucune généralisation aux effets distants ou au code synchrone non coopératif.
- services/Hive/infra/src/transaction.rs : analogue same-txn ; services/IAMRusty/migration/src/m20220101_000001_initial_schema.rs : migration partagée.
- crates/telegraph-events/src/lib.rs ; workers/sentinel-sync/src/handler.rs et translator/telegraph.rs : contrat et projection existants.
- docs/platform/events-outbox.md ; obsidian/AI FOR ALL/projects/telegraph/references/telegraph-event-processing.md : handbook/conception, pas ratification.
- Base historique analysée :93e376c5530a439aaafe61a64d3e881865c1acf1 ; elle n'est plus la réalité courante.

### Livraison vérifiée au 2026-10-28

- Application39024b2 intégrée à AIForAll master par66c16f1 ; SDK testé d60a88b
  intégré à RustyCog main par9e9ef27, gitlink mode160000 d60a88b conservé.
  [Vérification des deux intégrations](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/150-mission-et-sdk-integres-aux-branches-principales.md).
- [Composant après expiration](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/132-reprise-apres-expiration-prouvee.md) :1 PASS/0 échec, nouveau claim, vrai SQS, identités conservées et comptes1/0/1.
- [Verticale finale](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/139-validation-verticale-reussie.md) :1 PASS/0 échec/0 ignoré/6 filtrés ; nominal et panne/reconstruction, vrai SQS, handler Sentinel/OpenFGA, replay à ledger neuf, tuple unique,403 sans mutation puis200 destinataire ; comptes1/1/1. Son résiduel de lease était nul : le verrou actif reste la preuve distincte du composant.
- [Bilan et réserves](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/141-livraison-terminee-avec-reserves-explicites.md) : preuves transaction/NoOp/HTTP réutilisées avec leur portée, cause du timeout109 inconnue, Clippy strict antérieur non vert, Sonar/CI distante non exécutés. La publication application encore locale dans ce bilan est historique, levée par le rapport150.

Paramètres produit conservés : lease30s/poll2s/backoff5s/max20. La borne de preuve
après éligibilité est celle du contrat, pas une lease raccourcie ou un SLA nouveau.
0301 reste Accepted / Partial pour l'écart IAM ; pas de réparation historique,
déduplication IAM, exactly-once ou fencing multi-worker ajouté. Aucun nouveau
Cargo/runtime exécuté pour cette réconciliation documentaire.
