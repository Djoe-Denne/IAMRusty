---
title: >-
  Manifesto
category: project
tags: [projects, orchestration, blueprint, visibility/internal]
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/README.md
  - Manifesto/SETUP.md
  - Manifesto/IMPLEMENTATION_STATUS.md
  - Manifesto/src/main.rs
  - Manifesto/setup/src/app.rs
  - Manifesto/http/src/lib.rs
  - Manifesto/application/src/command/factory.rs
  - Manifesto/configuration/src/lib.rs
  - Manifesto/tests/common.rs
  - Manifesto/docs/rustycog-service-build-guide.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
  - Manifesto/docs/rustycog-implementation-and-usage-guide.md
  - docs/adr/0401-manifesto-projets-composants-acl-cas.md
summary: >-
  Manifesto : slice hexagonale (ADR 0401). Apparatus P0/P1/P2 Partial dans
  HEAD 7455ee5 ; Factory/host/gateway hors livré.
provenance:
  extracted: 0.76
  inferred: 0.22
  ambiguous: 0.02
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-09-13T10:25:00Z
---

# Manifesto

## Indexes

- [[projects/manifesto/concepts/index]] — concepts
- [[projects/manifesto/skills/index]] — skills
- [[projects/manifesto/references/index]] — references

Manifesto is the project-management service for AIForAll. Use `[[projects/rustycog/references/index]]` for the shared service shell and crate behavior; use this page and the linked Manifesto references for the project-domain rules, service-specific wiring, and current runtime truth.

## RustyCog Baseline

- `[[projects/rustycog/references/index]]` is the canonical map for the shared command, config, HTTP, permissions, DB, event, and testing crates that Manifesto composes.
- `[[references/rustycog-service-construction]]` and `[[skills/building-rustycog-services]]` describe the default RustyCog service assembly flow that Manifesto follows with project-specific additions.
- Read `[[projects/rustycog/references/rustycog-command]]`, `[[projects/rustycog/references/rustycog-config]]`, `[[projects/rustycog/references/rustycog-http]]`, `[[projects/rustycog/references/rustycog-permission]]`, and `[[projects/rustycog/references/rustycog-testing]]` for the shared baseline that this project specializes.

## Service-Specific Differences

- Manifesto treats projects as assemblies of independently implemented components with their own lifecycle, visibility, and configuration flow.
- The composition root is recognizably RustyCog-shaped, but Manifesto adds project-, component-, and member-scoped permission fetchers plus its own `ManifestoCommandRegistryFactory`.
- Live runtime now wires verified HS256-only auth, logging level, command retry, component-service timeout/api key, and business limits from config instead of leaving those knobs as guide-era leftovers.
- Public project/component resource reads use optional-auth routes plus a world-read use-case gate; private reads still require real access. Create-as-public and a later flip to public write `viewer@user:*`. `publish` does not — see [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]].
- Authenticated `GET /api/projects` SQL now matches GET for org-inherited access (Internal for org members, private for org admins). Anonymous list can still show live public rows without `user:*`; anonymous GET needs the wildcard.
- Public self-join is live (`POST /api/projects/{id}/join`, `Direct` / `project` / `read`). There is still no org partnership. `external_collaboration_enabled` and `MemberSource::OrgCascade` / `Invitation` are unused. Mutations require an active DB member or org admin ([[projects/manifesto/concepts/immediate-membership-acl]]).
- Component catalog integration is fail-closed.
- Component add/remove now treats component-instance ACL sync as part of the same consistency boundary and fails instead of silently drifting state.
- Apparatus status consumption is wired into startup when queue config resolves to a real consumer, while checked-in local/test configs keep queues disabled by default.
- `ComponentResponse.endpoint` and `access_token` still remain unset, so component provisioning handoff is the main product boundary that is still deliberately narrow.
- Partial project/member/component writes use [[concepts/optional-field-update]] (`FieldUpdate::Unchanged` vs `Set`) instead of nested `Option`.
- Manifesto remains the golden-path scaffold in [[concepts/architecture-coherence-across-services]] (logging désormais `setup_logging`, ADR 0100).

## Apparatus

Le rationnel reste [[projects/manifesto/concepts/apparatus-platform]]. **P0** : [[projects/manifesto/concepts/apparatus-p0-contracts]]. **P1** persistance : [[projects/manifesto/concepts/apparatus-p1-persistence]]. **P2** réconciliation in-process (Partial) : [[projects/manifesto/concepts/apparatus-p2-reconciliation]]. Factory / host / gateway restent hors livré (ADR 0406). HEAD `7455ee5`.

Les ADR vague 1 sont **Accepted** ([[projects/manifesto/decisions/index]]) avec `Réalité` **Partial**. La photographie du service actuel : ADR 0401 — [[projects/aiforall/decisions/0400-services-runtime]].

## Related

- [[projects/rustycog/references/index]] - Canonical shared framework map that the service pages below build on.
- [[references/rustycog-service-construction]] - Generic RustyCog construction flow that Manifesto specializes.
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]] - Ownership bootstrap, defaults, and publish/archive transitions.
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] - Org-owned visibility, join, list/GET, publish vs public.
- [[projects/manifesto/concepts/immediate-membership-acl]] - DB membership gate and Suspended matrix.
- [[projects/manifesto/concepts/membership-restore-and-cas]] - Grace restore, owner CAS, outbox in the same UoW.
- [[projects/manifesto/concepts/component-instance-permissions]] - Generic versus per-instance component permission model.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] - External component catalog integration and fail-closed behavior.
- [[projects/manifesto/references/manifesto-entity-model]] - Project, component, membership, and project-scoped RBAC entities.
- [[projects/manifesto/references/manifesto-api-and-permission-flows]] - Live route and permission behavior.
- [[projects/manifesto/references/manifesto-event-model]] - Events emitted and consumed by the service.
- [[projects/manifesto/references/manifesto-runtime-and-configuration]] - `MANIFESTO_*` config loading, queue posture, and runtime wiring.
- [[projects/manifesto/references/manifesto-testing-and-fixtures]] - DB-backed API harness plus focused runtime/auth/client tests.
- [[projects/manifesto/skills/extending-manifesto-project-service]] - Practical workflow for adding commands, routes, permissions, events, and tests.

## Open Questions

- Later work: L-PARTNERSHIP remains open on [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]]. Join, Internal, immediate ACL, and restore/CAS shipped 2026-09.
- Apparatus : P0 + P1 + P2 Partial dans `7455ee5` ; P2.1 (bump, writer, `/ready`) avant P3. Plan : [[projects/manifesto/references/apparatus-implementation-plan]]. ADR 0406.
- If queue-backed operation becomes more common outside local/test, should the checked-in config examples start surfacing explicit broker settings?

## Sources

- [[projects/manifesto/references/manifesto-service]] — Product model, runtime wiring, and project-service ADR summary
- [[projects/rustycog/references/index]] — Shared crate-level baseline for the runtime this service specializes
- [[references/rustycog-service-construction]] — Manifesto-authored RustyCog build and wiring guides
- [[skills/building-rustycog-services]] — Practical workflow distilled from those guides
