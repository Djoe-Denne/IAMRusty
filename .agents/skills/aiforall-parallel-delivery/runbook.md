# Delivery wave runbook

## Private ledger

Keep the ledger and raw receipts under a wave-specific, ignored `.tmp/`
directory. Keep secrets out of the ledger and published materials. Record
UTC times at patch-ready, slot admission, local outcome, push, and required
CI outcome so wait time does not look like agent time.

```text
Wave: <identity>; approved scope: <story revisions / examples>; base: <SHA>
Ownership: <lot, root session ID, verified agent=build and openai/gpt-6.2-sol#medium, absolute worktree, branch, shared surfaces>
Admission: <independent | sequenced after ... | blocked because ...>
Wake-up: <lot/phase, prompt ID and cutoff or exact revision/run, background job ID, log, deadline, next action>
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
   confirm that `build` is a primary profile (`GET /api/agent/build`) and
   that the OpenAI `gpt-6.2-sol` model and its `medium` variant are enabled
   in the native model catalogue (`GET /api/model`). If unavailable or
   quota-limited, report the blocker; never select a fallback on your own.
2. Create a top-level session with `POST /api/session`. Supply the explicit
   Build agent, OpenAI model with effort, and absolute worktree directory.
   **Do not omit `model`; omit `parentID`.** The native API uses a separate
   provider ID and unqualified model ID. OpenCode uses the parent's location
   for child creation, and that topology is wrong even if a child is
   subsequently moved to the right worktree.

   ```json
   {
     "title": "Build | STORY-021 | Sentinel Sync test pilot",
     "agent": "build",
     "model": {"providerID": "openai", "id": "gpt-6.2-sol", "variant": "medium"},
     "location": {"directory": "C:/absolute/repository/.worktrees/s021"}
   }
   ```

3. Persist the returned session ID and read it back with
   `GET /api/session/{sessionID}`. Require the same ID, `agent == "build"`,
   absent or null `parentID`, `location.directory` equal to the intended
   worktree after path normalization, and all three explicit `model` fields
   as above. A missing top-level `model` means the session can inherit a
   default; it does not pass. On resume, keep the existing owner. If its
   selection is missing or different, change the verified idle session
   using `POST /api/session/{sessionID}/model` with the same `model` object,
   then read it back. Never change an active run or send a prompt with
   an unverified model. If creation returns an uncertain result, reconcile
   the inventory so a lot does not acquire a second owner.
4. Confirm that the coordinator has the background completion mechanism
   described below. Record local UTC epoch milliseconds immediately before
   submitting each prompt; send it using `POST /api/session/{sessionID}/prompt`
   with a JSON `text` field and record the acknowledged prompt ID. A
   design-only lot receives design outputs and exclusions, not permission
   to change application behavior.
5. Verify that the current assignment has either returned and been handled,
   or has a live completion/deadline trigger owned by the coordinator before
   handing control back. An independent root session does not emit a
   child-completion notification to the coordinator.

If the installed client cannot create and verify root Build sessions, report
the missing capability and ask for supported access or user-created Build
sessions. Do not silently fall back to child profiles. If a prior wave
created specialists by mistake, preserve their work and any useful notes;
do not relabel, cancel, or remove those sessions without authorization.

## Mandatory wake-ups

Use OpenCode's background shell with completion delivery to the current
coordinator, not a detached shell process, a loop in an unrelated session,
or a note in the ledger alone. If completion delivery is not already known
to work in this coordinator, register a short no-op background job and
verify its notification first. If the client cannot deliver it, report
the blocker before dispatch.

After each accepted dispatch, launch this command with `background: true`
through the shell tool. Set a tool timeout longer than the wake interval.
Use the exact verified CLI path when it is not on `PATH`. Capture one
private log/receipt per wait and record the returned background job ID
against the current assignment. The default wake interval is three minutes;
use the user's agreed cadence when different. Track an escalation deadline
or no-progress threshold for each outstanding phase. No permanent timer is
installed.

```powershell
python .agents/skills/aiforall-parallel-delivery/scripts/wait_build.py --cli $cli --session $sessionId --since-ms $dispatchUtcMs --seconds 180
```

The helper listens to the native session wait endpoint or ends at the
deadline. Its `build-ended` result only reports the end of an execution
after the cutoff, not success. A queued prompt can briefly look idle; the
helper does not treat an older idle stamp as completion. A `deadline`
does not cancel the Build. `monitor-error` means the monitor could not
verify its target, not that the task or its tests failed.

**On each notification:**

- Match the recorded job, session, dispatch cutoff and current phase.
  Ignore superseded wakes. Read native state and the reply to the current
  assignment; `/api/session/active` alone does not prove a usable result.
  Record the exact handoff or why the assignment is still pending.
- For an ongoing task with no new reply, check its scope and agreed deadline.
  Re-arm one bounded wake if appropriate and record it; do not duplicate the
  prompt, waiter, or owner. At the escalation threshold, diagnose and report
  the concrete impediment rather than re-arming blindly. If tracking is
  lost, repair the watcher only after checking existing jobs. If delivery
  is blocked, record evidence, owner, and next permitted mitigation. Do not
  call a missing prerequisite or quota a passed check or a code defect.
- Re-enter the delivery plan once a worker returns. Start the next permitted
  validation in the background, recording its own job and receipt. For
  authorized publication, attach a completion notification to the exact
  expected GitHub run/attempt (for example, a background `gh run watch
  --exit-status`); then verify head/base/tree and the required check before
  merging. On failure, distinguish a product/test failure, an infrastructure
  incident, and incompatible evidence. Follow the existing retry and approval
  limits; do not retry an incomplete aggregation as if it were a code fix.
- Preserve wake coverage while work or checks are outstanding. Stop only
  after verification of the agreed deliverables (including merge and
  closeout when authorized), or with a reported blocker and a concrete
  resume condition. A design-only mandate ends at the agreed draft evidence,
  not at a silently broadened PR. Before exiting or resuming an interrupted
  wave, reconcile live jobs and receipts so an idle worker, finished timer,
  or stale hosted result cannot close or restart a lot.

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
- **Missing or wrong model:** A new Build inherits `opencode-go`, or
  reports OpenAI Sol at `high` or without a variant. Do not dispatch.
  Restore the exact selection on a verified idle owner and read it back.
  If the OpenAI connection cannot serve it, stop and report the blocker.
- **No notification path:** The client cannot deliver shell completion to the
  coordinator. Do not send a worker prompt or claim a monitored launch.
- **Fast, slow, and failed:** One Build finishes before its waiter starts,
  another is still busy at the deadline, and a third has a monitor error.
  Verify the first reply, re-arm only the ongoing assignment, and investigate
  the third without relaunching its worker.
- **Builds finished, PR pending:** All workers are idle after their handoffs,
  but the published PR still lacks its exact-revision gate. Follow the CI
  completion, classify failures, and do not close the wave on worker idle.
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
