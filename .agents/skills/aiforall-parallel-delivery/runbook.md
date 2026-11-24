# Delivery wave runbook

## Private ledger

Keep the ledger and raw receipts under a wave-specific, ignored `.tmp/`
directory. Keep secrets out of the ledger and published materials. Record
UTC times at patch-ready, slot admission, local outcome, push, and required
CI outcome so wait time does not look like agent time.

```text
Wave: <identity>; approved scope: <story revisions / examples>; base: <SHA>
Ownership: <story, session, absolute worktree, branch, shared surfaces>
Admission: <independent | sequenced after ... | blocked because ...>
Contract: <observable evidence, exclusions, migrations and integration order>
Exploration: <story, inspected SHA, absolute note path, passed to session>
Validation: <candidate SHA/tree, lane, exact command, result, receipt>
Runtime: <authorization, lease, fixture IDs, cleanup evidence>
Publication: <PR, head, base, tested tree, run, attempt, builds, required gate>
Closeout: <merge SHA/tree, archive verification, worktree/path/ref outcomes>
Resume: <next action, owner, evidence or approval still needed>
```

For an interrupted wave, verify live worktree, session and GitHub state before
acting. A terminal return, old check, or cached status does not prove a current
result. Pass opaque worker arguments structurally, not through shell-escaped
interpolated prose.

## Shared Cargo slot on Windows

The launcher uses atomic exclusive file creation at
`I:/cargo-target/AIForAll.qualification.lock`. Run it from the relevant
worktree with Python 3.10+; the launcher file may be read from the primary
checkout by older worktrees. Use a synchronous, reviewed command or script:

```powershell
python .agents/skills/aiforall-parallel-delivery/scripts/cargo_slot.py -- cargo check --locked
```

For a full turn, pass the approved validation script as one command. Run its
Cargo commands in order and wait for all test and cleanup processes. The
wrapper passes through an ordinary child exit code; it exits `75` on a busy
slot and `70` on an uncertain release. The path is outside every worktree.
This is a cooperative lock; direct commands that ignore this procedure are
not serialized. The host process check is additional admission protection,
not the lock. Hold the slot for the normal commit hook if it invokes Cargo.

If the owner disappears or cleanup cannot prove the host is idle, keep the
marker. Inspect its task, PID and worktree without executing its contents.
Only the primary session may authorize removal of that exact stale marker
after verifying the recorded owner and relevant children have stopped and
no successor could be using it. No age-based expiry, wildcard deletion, or
process termination to get the slot. Continue with authorized GitHub
qualification while the local slot is unavailable.

## Cases to rehearse without touching GitHub or running services

- **Shared seam:** Two approved stories touch one registry and one is blocked
  on an absent transition. Plan the integration owner and order. Do not
  start the blocked implementation or treat different worktrees as proof of
  independence.
- **Stale exploration note:** After a base update, a suggested edit point
  moved. Keep the verified observations, check the new location and the
  failing test, then record the revised approach. Do not change the contract
  or repeat the whole investigation.
- **Busy Cargo:** Hold the slot in one worktree and propose verification from
  another. Require the second to fail admission and use existing authorized
  CI for a publishable candidate. Do not call local checks passed.
- **Coverage retry:** A targeted CI rerun passes the shard but aggregation
  rejects old receipts. Keep the PR unqualified, preserve both attempts,
  and do not request a full rerun when only the shard was authorized.
- **Closeout:** A merged squash PR has ignored evidence and an initialized
  submodule. Verify its integrated contents, preserve and verify recovery,
  then remove only its authorized worktree. Keep an unrelated dirty tree.
  A locked empty path stays recorded as a pending filesystem cleanup.
