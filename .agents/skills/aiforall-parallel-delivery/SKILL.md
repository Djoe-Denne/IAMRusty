---
name: aiforall-parallel-delivery
description: Coordinate a requested AIForAll delivery wave for several approved stories or resume a wave across Build sessions, Git worktrees, local Cargo, GitHub CI, Sonar, and post-merge cleanup. Use for parallel story delivery or reproducing the multi-story delivery workflow. Not for product discovery, a standalone lint campaign, or unrelated worktree cleanup.
---

# AIForAll parallel delivery

Run this skill in the primary session, not in a new orchestrator. Keep
`AGENTS.md` and the [product delivery workflow](../../../docs/guides/product-delivery-workflow.md)
authoritative. Use the [ledger template and branch cases](runbook.md) to make
the wave resumable after an interruption. Loading the skill does not authorize
implementation, agent dispatch, infrastructure actions, or destructive cleanup.

## Admit and schedule the wave

1. Record the approved story revisions, acceptance examples, exclusions,
   approval boundaries, and freshly fetched `origin/master`. Reconcile each
   requested story with `aiforall-story-handoff`; distinguish existing
   evidence from new work. Do not dispatch implementation on a blocked
   prerequisite. Keep its owner, missing prerequisite, and resume condition.
2. Before splitting writers, map changed services plus shared crates, routes,
   migrations, events, configuration, and test fixtures/launchers. When
   delegation is authorized, request this cross-lot check from `architecte`
   before dependent tests or edits. Otherwise the primary session does it.
   Prefer stories on different services. Give shared surfaces one writer,
   reserve migration identifiers and registration order, and choose an
   integration sequence. Separate worktrees do not make overlapping contracts
   independent. Never rename an already integrated migration.
3. Assign authorized Build sessions one worktree each; record the exact
   session, branch, base, and absolute ledger path. Resume the same session
   for the same assignment. Pass the approved contract, scope, source
   identity, test expectations, and permitted actions. No nested delegation.
   Take security decisions first, then UX within the approved contract; ask
   about genuine product uncertainty or changes of mandate.
4. Check worker requests and revision changes on the agreed cadence (three
   minutes if requested), using completion notifications where possible.
   Notify the user on decisions, blockers, handoffs, or changed outcomes,
   not on every unchanged check. A worker going idle is not delivery.

## Qualify without queuing all work behind Windows Cargo

- The primary session owns admission to the local slot and runtime leases.
  Keep one Windows Cargo target, `I:\cargo-target\AIForAll`, and the configured
  24 internal jobs. Retain approved per-story build intermediates where
  needed; serialization alone did not prove that cross-worktree
  intermediates were safe.
- Use the [shared slot launcher](scripts/cargo_slot.py) for a bounded
  synchronous validation turn. Give the authorized owner a stable candidate
  and the whole turn (format check, named tests, and pertinent Clippy lanes),
  not one command at a time. Do not nest the launcher, detach commands from
  it, or bypass it with a plain Cargo invocation. The launcher does not grant
  runtime permissions. Follow `aiforall-test-evidence` and
  `local-runtime-lifecycle` when applicable.
- If the slot is busy, use the existing authorized GitHub PR CI on a clean,
  coherent, publishable commit rather than waiting for full native
  qualification. Label unrun checks **Not run locally**; do not bypass the
  normal commit hook, review, or merge gate. If a hook itself needs the slot,
  use an already committed candidate or wait for that hook. Keep moving on
  independent work rather than running a second local Cargo.
- Prove actual red and green, not an infrastructure failure. Use
  fixture-owned settings and test with ambient overrides absent, including
  missing-configuration negatives. Keep external acceptance boundaries
  separate: a database intent is not proof of a delivery at the recipient.
  Run the required independent reviews on the exact diff and refresh
  affected evidence after a correction.

## Publish, integrate, and close

- Run the standard formatting hook. For Clippy/Sonar findings, load
  `aiforall-sonar-policy`: freeze the current report, deduplicate distinct
  sites, fix them together, and use both production and tests lanes when
  local qualification is available. Use Sonar MCP for triage, not to
  submit scans. Preserve the authoritative CI/Sonar flow; no partial Sonar
  scan on the official PR, fabricated coverage, suppression, or gate changes.
- Bind conclusions to PR head, base/tested merge tree, run and attempt.
  Classify failures before considering a retry. A targeted GitHub retry can
  prove the failed shard recovered; it cannot satisfy a same-attempt
  coverage aggregator with old reports. Respect the authorized retry scope.
  When `master` advances, check semantic auto-merges as well as conflicts,
  preserve prior tests and migrations, and rerun affected checks. Require
  eight builds when selected and green `CI scope and result` on the exact
  current revision before merging.
- Start the local closeout immediately after each verified integration. Use
  only ledger-owned worktrees, preserve local evidence and any unique
  submodule state in a verified recoverable archive, confirm the remote
  branch state, then remove approved worktrees and branches under the
  existing cleanup rules. A squash merge needs content verification, not
  ancestry alone. Do not force an unapproved removal or prune globally.
  Check directory, worktree registration, and branch separately; report
  an empty but locked directory as a cleanup limitation, not a successful
  directory removal. Never touch open lots, the main checkout, or foreign
  work. Stop monitoring when no authorized deliverable or worker is active.
