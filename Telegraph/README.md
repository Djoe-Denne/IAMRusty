# Telegraph

Emails (welcome, reset mot de passe) et notifications in-app. Pas de flux SMS concret aujourd’hui.

- Préfixe HTTP : `/telegraph` (compose : port hôte **8081**)
- JWT consommateur : `[auth.jwt]` (aligné issuer/audience, commit `68ac628`)
- AuthZ : OpenFGA type `notification` (`recipient`)
- Consomme `telegraph-events` ; publie `notification_created` → `sentinel-sync-events`

## Lancer

```bash
cargo run -p telegraph-migration -- up
cargo run -p telegraph-service
```

Config : `Telegraph/config/` (`TELEGRAPH_*`). Routage event → mode dans `[queues.telegraph-events]`.

## Documentation

- Handbook : [`docs/services/telegraph.md`](../docs/services/telegraph.md), [`docs/functional/notifications.md`](../docs/functional/notifications.md)
- Events : [`docs/platform/events-outbox.md`](../docs/platform/events-outbox.md)
