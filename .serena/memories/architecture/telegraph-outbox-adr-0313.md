Jalon fiabilité Telegraph — docs/adr/0313-telegraph-notification-outbox-atomique.md ; Statut Proposed ; Réalité Unimplemented.
- Même transaction writer : notification, livraison éventuelle, NotificationCreated via recorder partagé.
- Disabled/degraded NoOp suspend publication sans faux published ; reprise conserve event_id.
- Contrat docs/contracts/telegraph-notification-outbox.md : AC1–AC7 ; sous-lot cancellation-safe SQS autorisé avant atomicité T2 ; budget QA 6 s proposé.
- Ne supersède pas 0301 ; ni exactly-once ni déduplication IAM. Voir le fichier ADR.