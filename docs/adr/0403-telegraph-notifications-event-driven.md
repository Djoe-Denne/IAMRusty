# ADR-0403 : Telegraph est event-driven ; son HTTP est étroit

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Emails de bienvenue / reset et notifications in-app doivent réagir aux événements d’identité sans ouvrir un « microservice mail » générique. La tentation est d’exposer un HTTP d’envoi (SMS, email arbitrable) ou de faire produire les mails par IAM.

Telegraph consomme déjà `iam-events` et n’expose que la lecture / l’acquittement des notifications.

## Décision

1. **Telegraph** est le bounded context **notifications**. Préfixe `/telegraph` (compose 8081). JWT consommateur `[auth.jwt]`.
2. **Pilotage event-driven** : processeurs infra (`email`, `notification`) + **`CommunicationFactory`** (descripteurs TOML → email / notification in-app). Telegraph **consomme `iam-events`** (`user_signed_up`, `user_email_verified`, `password_reset_requested`).
3. **HTTP étroit**, authentifié, limité à :
   - `GET /api/notifications`
   - `GET /api/notifications/unread-count`
   - `PUT /api/notifications/{id}/read`
   Check OpenFGA `notification` (recipient) ; tuples écrits par `sentinel-sync` sur `notification_created`.
4. **SMS n’est pas dans les routes live.** Le domaine / la config / les templates peuvent connaître `sms` ; `create_router` ne monte aucun endpoint d’envoi SMS (ni d’envoi email générique).

Publication : `notification_created` → `telegraph-events` / `sentinel-sync-events`.

## Conséquences

- IAM ne rend pas d’email ; il publie un contrat d’événement (0400, 0300).
- Un client ne POST pas « envoie ce SMS » sur `/telegraph`.
- Ajouter un canal live (SMS) exige une décision + des routes / processeurs, pas seulement un variant de template.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| HTTP d’envoi générique (email/SMS) | Élargit la surface ; le dépôt garde un read-model notifications |
| IAM envoie les mails | Duplique templates et secrets SMTP ; `CommunicationFactory` est chez Telegraph |
| SMS live « déjà prévu par la config » | `default_sms_provider` ≠ route montée |

## Non décidé ici

- Provider SMS réel, opt-in, et conformité.
- Unification du leaky re-export `iam_events` dans `telegraph_domain` (revue 2026).
- NATS / bus unique (hors dépôt actuel).

## Références

- Wiki / handbook : `docs/services/telegraph.md`, `docs/functional/notifications.md`, `docs/platform/events-outbox.md`
- Code : `Telegraph/http/src/lib.rs`, `Telegraph/domain/src/service/communication_factory.rs`, `Telegraph/infra/src/event/processors/`, `Telegraph/setup/src/app.rs`, `iam-events/`
- Preuve : tests `Telegraph/tests/*iam*` / `password_reset_requested_event_test.rs` ; 3 routes notifications seulement
