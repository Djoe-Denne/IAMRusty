# Telegraph

Emails (welcome, reset) et notifications in-app.

- Préfixe : `/telegraph` — compose : **8081**
- JWT : `[auth.jwt]` consommateur (aligné `68ac628`)
- OpenFGA : type `notification` (recipient)
- Consomme `telegraph-events` ; publie `notification_created` → `sentinel-sync-events`

## Docs

- [../functional/notifications.md](../functional/notifications.md)
- [../platform/events-outbox.md](../platform/events-outbox.md)
