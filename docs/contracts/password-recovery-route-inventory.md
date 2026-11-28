# Password recovery admission route inventory

Source baseline: `7b8660a1f263c918629450b9019b0ede0d32c84c`. This inventory
maps the route registrations for [OR-15](password-recovery-session-liveness.md).
Deployed outcomes must be recorded separately with their tested revision.
Multi-method registrations count once in the totals.

## IAM (`/iam`) — 12 required, zero optional

Source: [IAM HTTP router](../../services/IAMRusty/http/src/lib.rs).

- `GET`, `PATCH` `/api/me`
- `POST` `/api/me/totp/enrollment`, `/api/me/totp/enrollment/confirm`
- `DELETE` `/api/me/totp/enrollment/{enrollment_id}`
- `GET` `/api/me/email-addresses`
- `POST` `/api/me/email-verification/request`, `/api/me/email-verification/confirm`
- `GET` `/api/auth/{provider_name}/relink-start`, `/api/auth/{provider_name}/link`
- `POST` `/api/auth/password/reset-authenticated`
- `POST` `/internal/{provider_name}/token`
- `DELETE` `/internal/{provider_name}/revoke`

The two internal provider routes also require the workload token. The
`relink-callback` is state-authenticated; it does not install a prior bearer
session. Signup, login, verification, refresh, reset request, and reset
confirmation own their separate proofs. The liveness and checkpoint RPCs
have separate workload capabilities.

## Hive (`/hive`) — 21 required, one optional

Source: [Hive HTTP router](../../services/Hive/http/src/lib.rs).

- `GET`, `POST` `/api/organizations`
- `GET`, `PUT`, `DELETE` `/api/organizations/{organization_id}`
- `POST` `/api/organizations/{organization_id}/sync-jobs`
- `GET` `/api/organizations/{organization_id}/roles`, `/api/organizations/{organization_id}/roles/{role_id}`
- `GET`, `POST` `/api/organizations/{organization_id}/members`
- `GET`, `PATCH`, `DELETE` `/api/organizations/{organization_id}/members/{user_id}`
- `POST` `/api/organizations/{organization_id}/invitations`
- `DELETE` `/api/organizations/{organization_id}/invitations/{invitation_id}`
- `POST` `/api/invitations/{token}/accept`
- `POST` `/api/organizations/{organization_id}/external-links`
- `POST` `/api/organizations/{organization_id}/signer/configure`, `/api/organizations/{organization_id}/signer/test`, `/api/organizations/{organization_id}/signer/rotate`, `/api/organizations/{organization_id}/signer/disable`
- Optional: `GET` `/api/organizations/search`

## Manifesto (`/manifesto`) — 22 required, five optional

Source: [Manifesto HTTP router](../../services/Manifesto/http/src/lib.rs).

- `POST` `/api/projects`
- `PUT`, `DELETE` `/api/projects/{project_id}`
- `POST` `/api/projects/{project_id}/publish`, `/api/projects/{project_id}/archive`, `/api/projects/{project_id}/suspend`, `/api/projects/{project_id}/resume`, `/api/projects/{project_id}/transfer-ownership`, `/api/projects/{project_id}/join`
- `POST` `/api/projects/{project_id}/components`
- `PATCH`, `DELETE` `/api/projects/{project_id}/components/{component_id}`
- `GET`, `POST` `/api/projects/{project_id}/members`
- `GET`, `PUT`, `DELETE` `/api/projects/{project_id}/members/{user_id}`
- `POST`, `DELETE` `/api/projects/{project_id}/members/{user_id}/permissions/{resource}`
- `POST`, `DELETE` `/api/projects/{project_id}/members/{user_id}/permissions/{resource}/{resource_id}`
- `PUT` `/api/projects/{project_id}/bindings/{component_id}/consents`
- Optional: `GET` `/api/projects`, `/api/projects/{project_id}`, `/api/projects/{project_id}/details`, `/api/projects/{project_id}/components`, `/api/projects/{project_id}/components/{component_id}`

`GET /api/projects/{project_id}/bindings/{component_id}` uses a separate
workload HS256 verifier. Its workload test requires the real workload path
and should exercise both a valid workload credential and a rejected platform
credential.

## Telegraph (`/telegraph`) — three required, zero optional

Source: [Telegraph HTTP router](../../services/Telegraph/http/src/lib.rs).

- `GET` `/api/notifications`, `/api/notifications/unread-count`
- `PUT` `/api/notifications/{id}/read`

## Deployment paths and qualification

- **Direct and monolith:** IAM installs the same in-process authoritative
  checker in its own standalone and nested routers. The
  [monolith](../../runtime/monolith/src/runtime.rs) also nests the
  standalone Hive, Manifesto, and Telegraph routers. Those services obtain
  their HTTP liveness checker from the SDK's optional `auth.session_liveness`
  config. The baseline deployment templates do not set that config. The
  qualification target must explicitly wire the authoritative reader and
  prove the service-side positive, revoked, and unavailable outcomes.
- **Mesh:** [Compose](../../ops/deploy/mesh/envoy.yaml) and
  [Kind](../../ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml) send the
  four service prefixes through Check. The baseline templates deny
  requests without a bearer, including the six optional route registrations.
  The qualification change adds ordered exceptions for credential-absent
  GET and HEAD requests to those routes. Every presented credential,
  including an empty header, still goes to Check. The path and header
  matching comes from the
  [Envoy 1.31.2 route API](https://github.com/envoyproxy/envoy/blob/v1.31.2/api/envoy/config/route/v3/route_components.proto);
  real downstream outcomes remain to be qualified.
- **Hosted evidence:** The default campaign admits IAM, Ext Authz, and
  OAuthConnect. The optional same-repository preview also admits the real
  monolith and companion binaries. The
  [qualification report](../missions/20261128-password-recovery-replica-qualification.md)
  records tested revisions, route witnesses, and remaining gaps. A source
  registration alone is not evidence of deployed enforcement.
