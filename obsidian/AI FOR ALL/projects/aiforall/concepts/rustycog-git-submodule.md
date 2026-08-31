---
title: >-
  RustyCog git submodule
category: concepts
tags: [rustycog, git, ci, visibility/internal]
sources:
  - .agents/skills/rustycog-submodule/SKILL.md
  - Cargo.toml
  - cursor-conversation/github-ci-rustycog-2026-08-30
summary: >-
  rustycog/ is a pinned gitlink to Djoe-Denne/rustycog, patched over crates.io so local and GitHub CI resolve the same SDK commit.
provenance:
  extracted: 0.9
  inferred: 0.1
  ambiguous: 0.00
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# RustyCog git submodule

This workspace no longer vendors or gitignores a local rustycog fork. `rustycog/` is a `160000` gitlink to [Djoe-Denne/rustycog](https://github.com/Djoe-Denne/rustycog). Cargo still depends on crates.io `rustycog-framework` `0.1.1`, then overrides it:

```toml
[patch.crates-io]
rustycog-framework = { path = "rustycog" }
```

The patch is required because crates.io still ties `rdkafka` to the `events` / `full` features.

## Two trees

| Tree | Role |
|---|---|
| Sibling `../rustycog` | Develop and publish the SDK |
| `AIForAll/rustycog/` | Pinned checkout. Detached HEAD is normal |

Edits in the submodule are lost on the next `submodule update` unless they land in the rustycog repo first and this gitlink is bumped.

## Clone / CI

- Clone with `--recurse-submodules`, or `git submodule update --init rustycog`.
- GitHub CI must `checkout` the submodule so the workspace path patch resolves. Missing `rustycog/Cargo.toml` was a 2026-08-30 CI failure mode.

Portable pin/bump steps: `.agents/skills/rustycog-submodule/SKILL.md`. Crate APIs: [[skills/building-rustycog-services]] and [[projects/rustycog/rustycog]].
