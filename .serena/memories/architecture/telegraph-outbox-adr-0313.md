# Telegraph outbox — ADR0313

Canon : docs/adr/0313-telegraph-notification-outbox-atomique.md ; voir le fichier ADR.
Statut : Proposed. Réalité : Implemented (périmètre Telegraph), réconciliation2026-10-28.

- Notification, livraison éventuelle et intention NotificationCreated dans la même transaction writer ; aucun publish direct post-commit.
- Disabled/degraded NoOp suspend le dispatch sans faux published ; reprise durable, même événement.
- Application39024b2 intégrée master66c16f1 ; SDKd60a88b intégré main9e9ef27, pin160000 exact.
- Preuves distinctes : composant après lease132 et verticale139 SQS/OpenFGA/replay/403 sans mutation/200 ; rapport150 intégration, pas nouveau run.
- 0301 reste Accepted/Partial pour IAM ; pas Accept implicite, exactly-once, fencing/backfill ou quitus Clippy/Sonar/CI.
