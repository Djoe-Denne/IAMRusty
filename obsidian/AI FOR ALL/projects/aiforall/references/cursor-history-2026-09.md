---
title: >-
  Cursor history September 2026
category: references
tags: [reference, cursor, aiforall, manifesto]
sources:
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/54cbe718-8042-4a4e-8a15-767a7c0e15ba/54cbe718-8042-4a4e-8a15-767a7c0e15ba.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23280216-f48f-426e-8a11-6b42b8184a6e/23280216-f48f-426e-8a11-6b42b8184a6e.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/208e7f4b-556a-478d-80c2-dcdd79e8475b/208e7f4b-556a-478d-80c2-dcdd79e8475b.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/7d3c467c-eaeb-49f6-a6e3-f6b4ef5a68b2/7d3c467c-eaeb-49f6-a6e3-f6b4ef5a68b2.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/e9a1f9f4-d707-4a08-a607-638340586b89/e9a1f9f4-d707-4a08-a607-638340586b89.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/2924d20c-07e2-4741-a4aa-f3eb9e48c5ca/2924d20c-07e2-4741-a4aa-f3eb9e48c5ca.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/3c82c7d9-a42d-448f-8573-90ed739ba6b6/3c82c7d9-a42d-448f-8573-90ed739ba6b6.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/11c01523-bb74-444a-ac31-44384d63de7d/11c01523-bb74-444a-ac31-44384d63de7d.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/442685dd-30ba-4a9d-b59b-f8ae6aa47b9a/442685dd-30ba-4a9d-b59b-f8ae6aa47b9a.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d0c9b007-93a2-4cec-814a-155dcbb74d59/d0c9b007-93a2-4cec-814a-155dcbb74d59.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/5b38bc90-792f-4181-8fcd-11686b13f543/5b38bc90-792f-4181-8fcd-11686b13f543.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/9c672b13-9a03-4aeb-b35a-bf8a682f0bd6/9c672b13-9a03-4aeb-b35a-bf8a682f0bd6.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/688f5240-08ac-4f6e-94ce-323953bc759a/688f5240-08ac-4f6e-94ce-323953bc759a.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/345e4ffb-1efa-4ee7-bcd9-a6a769cf9516/345e4ffb-1efa-4ee7-bcd9-a6a769cf9516.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/e32d072c-e2be-42a4-82f4-2b6808695124/e32d072c-e2be-42a4-82f4-2b6808695124.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/8e0500f9-8cee-43b6-b67c-630f573a7877/8e0500f9-8cee-43b6-b67c-630f573a7877.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/f5e5a1e2-20b9-4794-95b0-c75b32a52fb4/f5e5a1e2-20b9-4794-95b0-c75b32a52fb4.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/6de9b375-2f3a-4301-9342-b9a00323c9a8/6de9b375-2f3a-4301-9342-b9a00323c9a8.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/cd391d48-2b6b-4300-8a70-452b2230a8d8/cd391d48-2b6b-4300-8a70-452b2230a8d8.jsonl
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/a5acce59-de4a-47c1-ae8b-cfd25a84c747/a5acce59-de4a-47c1-ae8b-cfd25a84c747.jsonl
summary: >-
  Cursor septembre 2026 : ADR, P0/P1/P2 Apparatus Partial (HEAD 7455ee5),
  vague 2 0100–0502, Grok-only.
provenance:
  extracted: 0.72
  inferred: 0.23
  ambiguous: 0.05
created: 2026-09-09T16:45:00Z
updated: 2026-09-13T10:25:00Z
---

# Cursor history September 2026

Follows [[projects/aiforall/references/cursor-history-2026-04-to-08]]. Parent jsonl only. Distilled by topic.

## Visibility and join (2–5 Sep)

- Public is world-read (`viewer@user:*`). `publish` is lifecycle only; `ProjectPublished` is a tuple no-op.
- `POST /api/projects/{id}/join` is live: public + active, `MemberSource::Direct`, grant `project`/`read`.
- Authenticated list/GET aligned for org Internal/admin. Anonymous list can show public SQL rows without the wildcard; GET still needs it.
- Red-team on uncommitted tests: do not fake sentinel-sync e2e inside Manifesto HTTP ITs. Component-catalog stubs use [[projects/rustycog/references/isolated-wiremock-fixture]].

## Manifesto AuthZ correction (9 Sep)

Durable contracts:

- [[projects/manifesto/concepts/immediate-membership-acl]]
- [[projects/manifesto/concepts/membership-restore-and-cas]]
- [[projects/sentinel-sync/concepts/manifesto-transport-and-ledger]]
- [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]]
- [[projects/manifesto/concepts/component-instance-permissions]] — `component_editor` / `component_viewer`, not `member from project`

A leftover OpenFGA Write without an **active** `project_members` row is 403. Tests that only `grant_write` without a member must expect 403.

## RustyCog pin and CI

- Develop SDK in the sibling repo on **`main`**. Detached HEAD in `AIForAll/rustycog/` is normal; do not leave SDK commits there.
- Manifesto `org_scope` needs `RelationshipTuple` and `OpenFgaPermissionChecker::read_tuples` **on that pin**. Checking out `main` without cherry-picking those APIs breaks Manifesto and Monolith CI.

## Sonar

Clippy/Sonar passes on unpushed files and a coverage-to-80% test plan ran the same day. Treat as quality work, not a product-rule change. ^[inferred]

## Apparatus ADR then P0 (10–11 Sep)

- Notes conception → ADR `docs/adr/0001`–`0005` **Accepted**, `Réalité` distincte. Hub [[projects/manifesto/decisions/index]].
- Triple review ADR vs wiki (cohérence / sécurité / implémentation, deux avis chacun). Les contre-reviews proposent des trajectoires minimales. ^[inferred]
- Harnais [[projects/aiforall/concepts/orchestrator-agent-harness]] : Muse trop coûteux remplacé par Grok Extra High pour le quotidien.
- P0 : [[projects/manifesto/concepts/apparatus-p0-contracts]]. Revue 2 Grok + 1 Muse → harness feature-gaté, puis 37 tests.

## P0 audit then P1 (11–12 Sep)

- Twin read-only P0 audits (Cursor + Codex, same verdict): P0 done with non-blocking reserves → P0.1. Standalone P1 prompt written (`docs/apparatus-p1-implementation-prompt.md`).
- P1 executed TDD-strict: duel gather → unique resolution → RED socle → P0.1 (42/42 + `apparatus-p0` CI job) → slices T1-T7 (36 tests). Later committed in `7455ee5`. See [[projects/manifesto/concepts/apparatus-p1-persistence]].
- ADR wave 1 enriched with P0.1/P1 proofs (`Partial` kept, `Accepted` untouched). Pages: [[projects/manifesto/decisions/index]].
- Model policy: Spark Max first in the morning, then user **banned Muse Spark** — Grok 4.6 Extra High only. See [[projects/aiforall/concepts/orchestrator-agent-harness]].
- P2 implementation prompt in preparation (Grok-only constraint up front).

## Vague 2 retrospective ADRs (12 Sep midday)

User asked for exhaustive retrospective ADRs (hexagon, crate responsibilities, IT, mocks vs real infra). Orchestrator + five `adr-*` agents (Grok Extra High) wrote `docs/adr/0100`–`0502`. Numbering is **by range**, not `0006` after Apparatus. Hub: [[projects/aiforall/decisions/index]].

Durable claims:

- Four business services = RustyCog vertical slices ; `setup` = only composition root ; commands registered by string key.
- IT = real HTTP + real DB ; WireMock = outbound HTTP only ; OpenFGA = real testcontainer. Producer queues opt-in ; Telegraph suite still queue-on (**Partial** 0202).
- `*-events` = contract, not transport. **NATS is not a transport.** Outbox same-txn Hive/Manifesto only (**Partial** 0301).
- Dual runtime standalones + `oodhive-monolith`. Apparatus crates = P0 + contrôleur P2 Partial (**Partial** 0406 pour Factory/host). Sonar `new_coverage` 80 % not met (**Partial** 0501).

## Apparatus P2 (12–13 Sep)

Parent sessions : `cd391d48` (prompt P2) puis revue `a5acce59`. Code + ADR-0006 dans HEAD `7455ee5` (working tree propre le 13 sept.).

- Ticker in-process Manifesto, migration `000013`, tests T1–T7. Pages : [[projects/manifesto/concepts/apparatus-p2-reconciliation]], [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]].
- Écarts A/D/I : pas de bump update/remove, pas de writer retry, `/ready` hors ticker. Mineurs (IF NOT EXISTS index, poison, fencing log) livrés.
- Prochain jalon **P2.1**, pas P3.

## Related

- [[journal/2026-09-13]]
- [[journal/2026-09-12]]
- [[journal/2026-09-11]]
- [[journal/2026-09-09]]
- [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]]
- [[projects/aiforall/concepts/rustycog-git-submodule]]
- [[projects/aiforall/concepts/orchestrator-agent-harness]]
