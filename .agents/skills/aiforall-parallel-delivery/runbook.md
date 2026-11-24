# Delivery wave runbook

## Private ledger

Keep the ledger and raw receipts under a wave-specific, ignored `.tmp/`
directory. Keep secrets out of the ledger and published materials. Record
UTC times at patch-ready, slot admission, local outcome, push, and required
CI outcome so wait time does not look like agent time.

```text
Wave: <identity>; approved scope: <story revisions / examples>; base: <SHA>
Ownership: <lot, root session ID, verified agent=build, absolute worktree, branch, shared surfaces>
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

## Independent Build sessions

Use the running OpenCode V2 instance, not the `subagent` tool. The documented
[OpenCode API](https://opencode.ai/docs/api#tag-session) is one supported
route: use the authenticated `opencode api` client and pass JSON
structurally. Perform these steps only for work packages authorized to start.

1. Prepare the lot's worktree and record its exact branch and base. Prefer the
   existing verified Build session when resuming. Before any new dispatch,
   confirm that `build` is a primary profile (`GET /api/agent/build`).
2. Create a top-level session with `POST /api/session`. Supply the explicit
   Build agent and absolute worktree directory. **Omit `parentID`.** OpenCode
   uses the parent's location for child creation and the topology is wrong
   even if a child is subsequently moved to the right worktree.

   ```json
   {
     "title": "Build | STORY-021 | Sentinel Sync test pilot",
     "agent": "build",
     "location": {"directory": "C:/absolute/repository/.worktrees/s021"}
   }
   ```

3. Persist the returned session ID and read it back with
   `GET /api/session/{sessionID}`. Require the same ID, `agent == "build"`,
   absent or null `parentID`, and `location.directory` equal to the intended
   worktree after path normalization. If any field differs, send no prompt.
   If creation returns an uncertain result, reconcile the session inventory
   before retrying so a lot does not acquire a second owner.
4. Send the authorized work package using `POST /api/session/{sessionID}/prompt`
   with a JSON `text` field. A design-only lot receives design outputs and
   exclusions, not permission to change application behavior. A queued
   response or an idle session is not a work result.
5. Track each recorded ID using native session status and replies. For API
   monitoring, use `GET /api/session/active` for running status and
   `GET /api/session/{sessionID}/message` for the reply to the current
   assignment. Native execution-finished notifications may replace scheduled
   checks; do not assume a root session will send a child-completion event
   to the coordinator. Keep the agreed cadence and stop when no lot is active.

If the installed client cannot create and verify root Build sessions, report
the missing capability and ask for supported access or user-created Build
sessions. Do not silently fall back to child profiles. If a prior wave
created specialists by mistake, preserve their work and any useful notes;
do not relabel, cancel, or remove those sessions without authorization.

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

- **Design lot, correct owner:** The initial set includes an OAuth architecture
  spike and a test pilot. Expect two root Build sessions with distinct worktrees;
  neither lot is assigned to a child `architecte` or test-engineer. A separately
  authorized specialist review does not count toward those two owners.
- **Wrong session shape:** The returned session has `agent: "architecte"`, a
  `parentID`, or a different worktree. Reject it before sending a prompt, keep
  any existing work, and report the mismatch rather than starting a substitute.
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
