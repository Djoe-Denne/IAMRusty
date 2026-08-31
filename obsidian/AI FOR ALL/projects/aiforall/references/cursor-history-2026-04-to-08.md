---
title: >-
  Cursor history April–August 2026
category: references
tags: [reference, cursor, aiforall]
sources:
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts
summary: >-
  Distilled Cursor parent sessions from 2026-04-26 to 2026-08-31: Sonar lanes, rustycog pin, architecture reviews, readiness, and wiki ops.
provenance:
  extracted: 0.55
  inferred: 0.40
  ambiguous: 0.05
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# Cursor history April–August 2026

Knowledge compiled from 29 parent Cursor sessions after the last full wiki wave (2026-04-26). Subagent transcripts were skipped. Distilled by topic, not by chat.

## Quality and CI

- Clippy reports were wired into GitHub Actions and Sonar (`clippy.json`, `sonar-project.properties`). Unit-test CI plus LCOV was added alongside the existing `tests/` integration lane.
- `cargo fmt --all -- --check` is enforced via `.githooks/pre-commit` (`docs/CARGO_FMT_PRE_COMMIT.md`).
- 2026-08-30: GitHub failed because rustycog was not checked out. The pin is now a submodule — [[projects/aiforall/concepts/rustycog-git-submodule]].
- 2026-08-30/31: large Sonar backlogs were closed with file-disjoint agent lanes — [[projects/aiforall/skills/running-parallel-sonar-lanes]].

## Platform reviews (2026-08-29)

- Parallel architecture reviews of IAMRusty, Manifesto, Telegraph (“Telegraf”), then a comparison doc at `docs/reviews/iam-architecture-comparison.md`.
- Follow-up slices: JWT/JWKS unification, Hive route/registry parity, `/ready` queue signaling. See [[concepts/architecture-coherence-across-services]].

## Events, authz, tests

- SQS fanout and outbox questions (already on the April roadmap) stayed the mental model for “one event, several services”.
- Integration tests were slow because containers restarted per test; reuse / singleton fixtures remain the recommended path ([[skills/creating-testcontainer-fixtures]]).
- Sentinel Sync gained a real `begin` / `complete` / `fail` ledger so failed OpenFGA writes stay retryable.

## Tooling

- Local Serena + GrepAI were scoped to this repo only (no global MCP rewrite, no multi-repo GrepAI workspace).
- QMD collection for this vault is `aiforall-wiki` (from the project `.env`).

## Codex

Seventeen Codex rollouts mentioned AIForAll; several ran with `cwd` on this repo (2026-08-28/29). They were inventoried, not re-quoted. Treat them as supporting evidence for the same themes (CI, Hive, IAM), not a second source of record. ^[inferred]
