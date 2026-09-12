# ADR-0300 : Les crates `*-events` du workspace sont le contrat, pas le transport

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Les bounded contexts échangent des faits de domaine (IAM → Telegraph, Hive / Manifesto / Telegraph → sentinel-sync). La tentation est de coupler le schéma d’événement au bus (SQS, Kafka, un client NATS) ou de traiter toute crate `*-events` du disque comme membre du workspace.

Le handbook `docs/platform/events-outbox.md` sépare déjà producteurs, files physiques et consommateurs. `QueueConfig` (rustycog-config) est le seul sélecteur de transport.

## Décision

1. **`iam-events`**, **`hive-events`**, **`manifesto-events`** et **`telegraph-events`** sont les crates de **contrat** (types / enveloppes de domaine). Elles ne portent pas le transport.
2. Le transport se configure via **`QueueConfig`** : `Sqs` | `Kafka` | `Disabled`. Les TOML test/dev mettent `queue.enabled = false` par défaut (files opt-in, voir 0202).
3. **`apparatus-events`** existe sur disque mais **n’est pas** dans `workspace.members` — hors contrat workspace de cette ADR.

La file physique (`telegraph-events`, `sentinel-sync-events`, préfixe `test-` en IT) est un câblage d’opérateur, pas le schéma.

## Conséquences

- Un producteur dépend de sa crate `*-events` + `QueueConfig` ; il n’importe pas un client de bus dans le contrat.
- `Disabled` est un transport valide : pas de publication, le contrat reste compilable.
- Ajouter un event AuthZ-relevant sans bras `sentinel-sync` laisse OpenFGA à la dérive (0303).
- Ne pas promouvoir `apparatus-events` au workspace sans ADR dédiée.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Contrat + client SQS/Kafka dans la crate `*-events` | Le transport est déjà `QueueConfig` ; le contrat doit rester portable |
| Une crate events unique pour toute la plateforme | Quatre bounded contexts, quatre contrats ; le fan-out SQS est 1:1 par file |
| Traiter `apparatus-events` comme membre implicite | Absent de `Cargo.toml` `[workspace].members` |

## Non décidé ici

- **NATS** : absent de `QueueConfig` et du dépôt. Pas un transport plateforme.
- Fan-out multi-consommateurs au-delà des files déjà déclarées (`[queue.queues]` ou worker dédié).
- Admission / host Apparatus et tout schéma `apparatus-events`.

## Références

- Handbook : `docs/platform/events-outbox.md` (producteurs → files, consommateurs, fan-out)
- Code : `Cargo.toml` `members` (`iam-events`, `hive-events`, `manifesto-events`, `telegraph-events` ; pas `apparatus-events`)
- Transport : `rustycog/rustycog-config/src/lib.rs` (`QueueConfig::{Kafka, Sqs, Disabled}`)
- Preuve : crates events dans le workspace ; `sentinel-sync` consomme via `create_event_consumer_from_queue_config` ; `apparatus-events/` hors members
