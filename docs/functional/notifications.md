# Parcours notifications (Telegraph)

Telegraph envoie des emails et stocke des notifications in-app. Préfixe `/telegraph`. Compose : 8081.

## Entrée (events IAM)

File `telegraph-events` (IAM produit) :

| Event | Mode | Effet |
|---|---|---|
| `user_signed_up` | email | Mail de bienvenue (template `user_signed_up`) |
| `password_reset_requested` | email | Mail reset |
| `user_email_verified` | notification | Ligne in-app (dev/test) |

Le routage fin est dans `[queues.telegraph-events]` (pas le contrat rustycog générique). SMS : mentionné dans d’anciens README, **pas** un flux concret actuel.

## Sortie HTTP (in-app)

Toutes authentifiées :

- `GET /telegraph/api/notifications`
- `GET /telegraph/api/notifications/unread-count`
- `PUT /telegraph/api/notifications/{id}/read` — `Write` sur type `notification`

Le tuple `notification:{id}#recipient@user:{user_id}` est écrit par **sentinel-sync** à `notification_created`. Sans worker, mark-as-read → 403 même pour le destinataire métier.

## Infra mail

IT : MailHog testcontainer (skill fixtures). Compose local : selon `Telegraph/config/development.toml` (SMTP).

## Suite

- [identite.md](identite.md) pour les événements source
- [../services/telegraph.md](../services/telegraph.md)
