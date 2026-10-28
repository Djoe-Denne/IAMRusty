# ADR-0301 : La publication durable passe par l’outbox transactionnel rustycog

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Suivi factuel — Telegraph livré, 2026-10-28

Les sections suivantes conservent la photographie rétroactive du2026-09-12,
pas une description du Telegraph courant. Depuis [0313](0313-telegraph-notification-outbox-atomique.md)
(Proposed / Implemented), notification, livraison éventuelle et intention
NotificationCreated partagent la transaction writer Telegraph. Le transport
disabled/degraded suspend le dispatcher sans faux published ; reprise après
lease, vrai SQS, projection OpenFGA, replay et droits403/200 sont prouvés dans le
périmètre de la livraison. Voir son [contrat](../contracts/telegraph-notification-outbox.md)
et le [contrôle d'intégration aux branches principales](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/150-mission-et-sdk-integres-aux-branches-principales.md).

**0301 reste Accepted / Partial** : l'écart IAM de transaction séparée n'est pas
résolu. Les mentions « Telegraph sans outbox » et « introduire un outbox » ci-dessous
sont historiques ; elles ne nient pas la livraison ultérieure et ne ratifient pas
0313. Pas de réécriture de la décision rétroactive, de backfill ou de garantie
exactly-once implicite.

## Contexte

Publier directement sur la file après un `COMMIT` métier ouvre « persist OK / publish perdu ». Inversement, prendre la file pour source de vérité inverse le modèle : le bus n’est qu’un transport (0300).

`rustycog-outbox` documente l’intention : enregistrer l’événement dans la **même transaction** que l’agrégat, puis un poller publie (SQS). Voir `docs/platform/events-outbox.md`.

## Décision

1. La publication **durable** d’un event de domaine = **outbox rustycog**, pas un `send` best-effort sur la file.
2. **Intention same-txn** : Hive et Manifesto enregistrent l’outbox dans la transaction de l’agrégat (`OutboxRecorder` + unit of work).
3. **IAM** a un outbox, mais dans une **transaction séparée** (`IamOutboxUnitOfWorkImpl::record_event` ouvre son propre `begin_write_transaction`).
4. **Telegraph n’a pas d’outbox** (consommateur / producteur `notification_created` hors ce motif).
5. **La file n’est pas la vérité** : elle transporte ; l’agrégat (+ outbox le cas échéant) reste la source.

`max_attempts = 0` sur `[command.retry]` désactive les retries ; ce n’est pas un défaut infini.

## Conséquences

- Hive / Manifesto : un rollback métier annule aussi l’enregistrement outbox.
- IAM : un succès métier peut encore perdre l’event si la txn outbox échoue (écart vs intention same-txn).
- Telegraph : pas de garantie outbox pour `notification_created`.
- Les IT ne doivent pas traiter l’absence de message SQS comme preuve d’absence d’état métier.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Publish-after-commit sans outbox | Perte d’event après persist ; le handbook l’écarte |
| File = source de vérité | Transport `QueueConfig` (0300) ; SQS est 1:1, pas un journal métier |
| Imposer l’outbox à Telegraph dans cette photo | Le code n’a pas le motif ; ce n’est pas livré |

## Non décidé ici

- Aligner IAM sur la same-txn Hive/Manifesto.
- Introduire un outbox Telegraph.
- Poller, rétention et destaging de la table outbox au-delà de rustycog-outbox.

## Références

- Handbook : `docs/platform/events-outbox.md` (section Outbox)
- Wiki rustycog : `projects/rustycog/references/rustycog-outbox`
- Same-txn : `services/Hive/infra/src/transaction.rs` (`HiveOutboxUnitOfWorkImpl`), `services/Manifesto/infra/src/transaction.rs` (`ProjectAuthorizationUnitOfWorkImpl` + `outbox.record(&txn, …)`)
- Txn séparée : `services/IAMRusty/infra/src/transaction.rs` (`record_event` → `begin_write_transaction`)
- Preuve Partial : outbox Hive/Manifesto/IAM + tests `services/Hive/tests/outbox_tests.rs`, `services/IAMRusty/tests/outbox_tests.rs`, `services/Manifesto/tests/apparatus_p1_t4_outbox.rs` ; Telegraph sans crate/outbox
