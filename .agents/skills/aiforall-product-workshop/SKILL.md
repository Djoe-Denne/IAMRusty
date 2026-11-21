---
name: aiforall-product-workshop
description: Facilitate or resume an AIForAll product discovery and Example Mapping workshop. Record desired behavior before inspecting implementation, save private interview notes, and create an approved public story through the Backlog.md CLI only when authorized.
---

# AIForAll product workshop

Read `docs/guides/product-story-workflow.md` for storage, tooling and approval
rules. Facilitate in the current conversation; do not start a worker or treat a
workshop as an implementation mission.

1. Pick one actor, goal and outcome. Place it in the journey using Story
   Mapping. Ask one decision at a time. Use a concrete example, counterexample
   or open question to expose an ambiguity. Do not infer an answer from code,
   an audit or existing documentation, including via QMD. Distinguish proposals
   from decisions the user actually made.
2. Save a brief interview note at `.tmp/product-workshops/<slug>/intent.md` in
   the primary checkout as decisions accumulate. Include scope, agreed rules,
   numbered observable examples, explicitly deferred behavior, open questions
   and the next question. Do not save raw chat, secrets or personal data.
   Resume from this file.
3. Restate the proposed intent and ask the user to approve that exact revision.
   Keep approval local if public wording has not been approved. Do not start
   implementation or change the intent to fit observed behavior.
4. When publishing wording is authorized, use `backlog task create` or
   `backlog task edit` for the one canonical story. Use a single-line `--ac`
   for each approved example, and link the relevant documentation rather than
   copying it. Capture the actual story ID and verify with
   `backlog task view <id> --plain`. No direct edits to Backlog files,
   files containing private facts in the vault, or changes to Git settings.
5. If requested, switch to `aiforall-story-handoff` for a separate comparison.
   Treat this as evidence gathering, not a continuation of product discovery.
