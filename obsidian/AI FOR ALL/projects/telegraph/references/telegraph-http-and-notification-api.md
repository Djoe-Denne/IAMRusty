---
title: Telegraph HTTP and Notification API
category: references
tags: [reference, api, notifications, visibility/internal]
sources:
  - Telegraph/openspecs.yaml
  - Telegraph/http/src/lib.rs
  - Telegraph/http/src/handlers/notification.rs
  - Telegraph/http/src/handlers/communication.rs
  - Telegraph/resources/permissions/notification.conf
  - Telegraph/application/src/usecase/notification.rs
  - Telegraph/domain/src/service/notification_service.rs
  - Telegraph/domain/src/service/permission_service.rs
summary: Source-backed view of Telegraph's JWT-protected notification API, ownership checks, and the gap between the live route table and broader communication DTOs.
provenance:
  extracted: 0.72
  inferred: 0.14
  ambiguous: 0.14
created: 2026-04-14T18:18:24.0602572Z
updated: 2026-04-14T18:18:24.0602572Z
---

# Telegraph HTTP and Notification API

These sources describe the live synchronous API surface of `[[projects/telegraph/telegraph]]`: authenticated notification listing, unread counts, mark-read behavior, and the permission model that keeps users scoped to their own stored notifications.

## Key Ideas

- The OpenAPI file documents `GET /api/notifications`, `GET /api/notifications/unread-count`, `PUT /api/notifications/{id}/read`, and `GET /health` as the live Telegraph surface.
- `http/src/lib.rs` wires those three notification endpoints through `rustycog_http::RouteBuilder`, marks them authenticated, and adds write permission checks to the mark-read route.
- Notification handlers build typed commands, attach `CommandContext::with_user_id()`, and map validation, business, not-found, unauthorized, and internal failures into HTTP status codes.
- `NotificationUseCaseImpl` handles pagination defaults, `per_page <= 100`, unread filtering, and response shaping for the notification read model.
- Ownership is enforced twice: the `ResourcePermissionFetcher` only grants `Permission::Write` when the user owns the notification ID, and `NotificationServiceImpl::mark_notification_as_read()` returns an unauthorized domain error if the record belongs to someone else.
- `http/src/handlers/communication.rs` defines richer direct-send DTOs for email, notification, and SMS payloads, but the live route table does not register those handlers. Conflict to resolve. ^[ambiguous]

## Open Questions

- The OpenAPI contract presents Telegraph as a notification service with real-time SQS processing, while the live HTTP server exposes only the read-model half of that broader story. ^[ambiguous]
- `RouteBuilder::health_check()` likely provides the `/health` endpoint described in OpenAPI, but the service does not define a dedicated Telegraph-specific health handler in this crate. ^[inferred]

## Sources

- [[projects/telegraph/telegraph]] - Project page for the service exposing these routes.
- [[projects/telegraph/concepts/multi-channel-delivery-modes]] - Broader communication DTOs versus the live notification-only routes.
- [[projects/telegraph/references/telegraph-service]] - Service shape and `rustycog_http` routing context.
- [[projects/telegraph/references/telegraph-testing-and-smtp-fixtures]] - Integration tests that exercise the live API.
