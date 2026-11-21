---
name: aiforall-story-handoff
description: Reconcile an explicitly approved AIForAll product story with an identified Git revision, record evidence per acceptance example, and prepare a read-only Product Manager handoff for requested delivery. Not for unapproved product discovery or autonomous implementation.
---

# AIForAll story reconciliation and handoff

Read `docs/guides/product-story-workflow.md`. Confirm which intent revision the
user approved and which code revision is being examined. Prefer
`backlog task view <id> --plain` in the relevant checkout for a published
story, or the approved local snapshot in `.tmp/product-workshops/`. A board
status, old search result or retroactive story map is not approval.

- Keep accepted intent fixed. Only now consult QMD, documentation, source and
  existing tests. For Rust exploration follow `AGENTS.md` and load
  `aiforall-mcp-workflow` first. Use the appropriate evidence and runtime rules
  if separate authorized test work is needed.
- For each numbered example record **Matches**, **Differs** or **Unverified**,
  the source/test path, observed result and what the evidence does not prove.
  Distinguish a source reading or historical test from a fresh executed test.
  Record the precise Git revision and any relevant local diff. Do not check off
  a criterion on an inference alone.
- Store the comparison separately from the story. Keep private work in the
  interview directory. For an approved public report follow the guide and
  attach it via the Backlog CLI, never by editing generated Markdown.
- Report mismatches as observations. If a mismatch raises a product question,
  take it back to the workshop. A revised story needs new approval and does
  not rewrite past evidence.
- When the user asks for planning, pass the PM the story ID or snapshot,
  approval revision, example IDs, evidence paths, verified gaps and the
  authorized planning scope, including what is excluded. If scope or approval
  is missing, ask for it. If no work is needed, return that conclusion without
  creating a ticket. Do not create delivery tasks or start implementation
  unless separately requested.
