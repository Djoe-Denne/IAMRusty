# Événements, files et outbox

Les services publient des événements domaine (crates `*-events`). Le routage SQS est **par type d’événement**, pas un bus unique.

## Producteurs → files (dev)

| Producteur | Event types | File physique |
|---|---|---|
| IAMRusty | `user_signed_up`, `user_email_verified`, `password_reset_requested` | `telegraph-events` |
| Hive | `organization_*`, `member_joined`, `member_removed`, `member_roles_updated` | `sentinel-sync-events` |
| Manifesto | `project_created`, `project_visibility_changed`, `project_published`, `project_archived`, `component_*`, `member_*`, `permission_*` | `sentinel-sync-events` |
| Telegraph | `notification_created` | `sentinel-sync-events` |

Tests : mêmes clés, préfixe `test-` (`test-telegraph-events`, `test-sentinel-sync-events`). `queue.enabled = false` dans les TOML test/dev par défaut — activer explicitement (`HIVE_QUEUE__ENABLED=true`, etc.) pour les suites transport.

## Consommateurs

- **Telegraph** lit `telegraph-events` et mappe via `[queues.telegraph-events]` : `user_signed_up` / `password_reset_requested` → email ; `user_email_verified` → notification in-app.
- **sentinel-sync** lit `sentinel-sync-events` (ou sa config) et dispatch par préfixe `event_type` vers `HiveTranslator`, `ManifestoTranslator`, `IamTranslator`, `TelegraphTranslator`.
- **Manifesto** peut aussi *consommer* `component_status_changed` (apparatus) si la queue est réellement résolue.

Un événement sans bras translator : `None` — pas d’erreur, **pas de tuple**. Ajouter un event AuthZ-relevant sans mettre à jour `sentinel-sync/src/translator/` = store FGA à la dérive.

## Traductions FGA (essentiel)

- Hive : rôles → `owner` / `admin` / `member` / `viewer` sur `organization`. Permission métier `write` → relation `member` ; `read` → `viewer`. Delete org : delete de toutes les relations connues pour owner + membres **portés par l’événement** (le translator ne relit pas la DB).
- Manifesto : membership + `project:{id}#viewer@user:*` si `visibility = public`.
- Telegraph : `notification:{id}#recipient@user:{user_id}` sur `NotificationCreated`.

## Outbox

`rustycog-outbox` : écrire l’événement dans la **même transaction** que l’agrégat, puis un poller publie sur SQS. Évite « persist OK / publish perdu ». Voir wiki `projects/rustycog/references/rustycog-outbox`.

`max_attempts = 0` sur `[command.retry]` **désactive** les retries (ce n’est pas « défaut infini »).

## Fan-out

SQS est 1:1. Pour N consommateurs, déclarer N files dans `[queue.queues]` pour le même `event_type`, ou un worker dédié. Ne pas compter sur une file unique partagée.

## Suite

- Skill : [`.agents/skills/rustycog/references/using-rustycog-events.md`](../../.agents/skills/rustycog/references/using-rustycog-events.md)
- [authz-openfga.md](authz-openfga.md)
- [../services/sentinel-sync.md](../services/sentinel-sync.md)
