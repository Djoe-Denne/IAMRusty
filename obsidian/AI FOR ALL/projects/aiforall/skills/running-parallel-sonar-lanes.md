---
title: >-
  Running parallel Sonar lanes
category: skills
tags: [sonar, clippy, skill, visibility/internal]
sources:
  - .agents/skills/aiforall-sonar-policy/SKILL.md
  - rustycog/.cursor/skills/rustycog-sonar-parallel/SKILL.md
  - cursor-conversation/sonar-eight-agents-2026-08-31
  - sonar-project.properties
summary: >-
  Split current Sonar issues into file-disjoint lanes (Cargo package × src|tests), one agent per lane, no workspace cargo while others edit.
provenance:
  extracted: 0.84
  inferred: 0.14
  ambiguous: 0.02
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# Running parallel Sonar lanes

Use this when closing a large Sonar backlog on `Djoe-Denne_IAMRusty` (or the rustycog-framework campaign) with several agents at once.

## Split

- Fetch **current** OPEN/CONFIRMED issues from the Sonar MCP. Do not reuse a stale CSV as the source of truth.
- Partition by file so two agents never edit the same path.
- Mutex = Cargo package × `{src|tests}`. A tests lane and a src lane on the same crate can still collide on generated code — keep them sequential if unsure. ^[inferred]
- Aim for enough lanes to keep agents busy (eight was the 2026-08-31 target) without crossing crate boundaries mid-lot.

## While agents run

- No `cargo` workspace build in Docker (or a shared `target/`) while other lots edit.
- After a lot: `cargo check -p <crate>` (and `--tests` if the lane is tests). Targeted Clippy `-W clippy::future_not_send` / `-W clippy::too_many_lines` on touched files.
- Do not `change_sonar_issue_status` unless it is a real false positive.

## Policy

Judgment and invalidated skips live on [[projects/aiforall/skills/fixing-sonar-clippy-in-services]]. Service work uses `.agents/skills/aiforall-sonar-policy/SKILL.md`. SDK work uses `rustycog/.cursor/skills/rustycog-sonar-parallel/SKILL.md`.

## Related

- [[projects/aiforall/concepts/rustycog-git-submodule]]
- [[skills/using-rustycog-core]]
