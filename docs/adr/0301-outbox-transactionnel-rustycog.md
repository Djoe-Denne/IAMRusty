# ADR-0301 : La publication durable passe par l’outbox transactionnel rustycog

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

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
- Same-txn : `Hive/infra/src/transaction.rs` (`HiveOutboxUnitOfWorkImpl`), `Manifesto/infra/src/transaction.rs` (`ProjectAuthorizationUnitOfWorkImpl` + `outbox.record(&txn, …)`)
- Txn séparée : `IAMRusty/infra/src/transaction.rs` (`record_event` → `begin_write_transaction`)
- Preuve Partial : outbox Hive/Manifesto/IAM + tests `Hive/tests/outbox_tests.rs`, `IAMRusty/tests/outbox_tests.rs`, `Manifesto/tests/apparatus_p1_t4_outbox.rs` ; Telegraph sans crate/outbox
