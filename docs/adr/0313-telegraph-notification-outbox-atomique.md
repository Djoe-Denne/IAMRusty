# ADR-0313 : Telegraph enregistre NotificationCreated dans la transaction de création de notification

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-10-25
- Décideurs : à renseigner après ratification humaine du texte
- Jalon concerné : fiabilité Telegraph, hors P-Apparatus
- SuperSède : aucune
- SuperSédée par : —

Accepted ratifie une cible ; Réalité décrit séparément le code.

## Contexte

L'ADR [0301](0301-outbox-transactionnel-rustycog.md), Accepted / Partial,
photographie Telegraph sans outbox et laisse son introduction ouverte. Les deux
créations de NotificationServiceImpl persistent puis appellent
publish_notification_created : une panne transforme un succès métier en erreur
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

- services/Telegraph/domain/src/service/notification_service.rs : deux créations et publication post-persistance.
- services/Telegraph/infra/src/repository/notification_write.rs ; domain/src/port/repository.rs : frontière actuelle.
- services/Telegraph/setup/src/app.rs ; runtime/monolith/src/runtime.rs : handles et drain actuels.
- rustycog/rustycog-events/src/sqs.rs : ownership pollers déficient au pin 6c63b65c23e26b8f83f1c6f63d656ef31c7591d0 confirmé par le lead, constat statique.
- services/Hive/infra/src/transaction.rs : analogue same-txn ; services/IAMRusty/migration/src/m20220101_000001_initial_schema.rs : migration partagée.
- crates/telegraph-events/src/lib.rs ; workers/sentinel-sync/src/handler.rs et translator/telegraph.rs : contrat et projection existants.
- docs/platform/events-outbox.md ; obsidian/AI FOR ALL/projects/telegraph/references/telegraph-event-processing.md : handbook/conception, pas ratification.
- Preuve d'implémentation : aucune ; base analysée 93e376c5530a439aaafe61a64d3e881865c1acf1. 0301 reste inchangée, notamment son écart IAM.
