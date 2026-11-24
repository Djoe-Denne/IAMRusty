# AIForAll product delivery workflow

## Refine and select work

1. Agree on a product story and numbered observable examples with the user
   through the [product story workflow](product-story-workflow.md).
2. `project-manager` compares the approved intent with code and tests, checks
   reusable designs and external options, and proposes slices. Record the
   chosen local option, rationale, alternatives, known gaps, verification,
   and dependencies in each authorized Backlog delivery task. Cite the parent
   story and example IDs, plus the applicable ADR and implementation contract.
   Record a library or third-party service's costs and implications rather
   than silently adding it.
3. Among unblocked tasks, choose the best value for the effort first. When
   there are no meaningful quick wins, select the work that unlocks the most
   valuable downstream tasks. Do not start a task because it has a high score
   or many dependents if it is still blocked.

## Design, then test

4. `architecte` chooses the technical solution when open, not just the API.
   A product workshop's suggested architecture is an option to evaluate.
   Complex work or a change to architecture gets a `Proposed` ADR and any
   required contract in [`docs/contracts/`](../contracts/README.md). Refer to
   existing contracts where sufficient. Before dependent tests, agree on
   the observable API, event, or equivalent boundary. Product changes go
   back to the user; a proposed ADR is not self-ratified by its author.
5. `test-engineer` chooses the simplest sufficient proof and writes tests
   against the agreed contract. For user-facing slices, prove the failure
   with integration tests and the affected end-to-end journey. Reuse the
   project's fixtures and obey ADR 0200–0202. Keep the command, revision,
   and expected failing assertion. Infrastructure failure is not red.
   Use an authorized runtime only; do not start Kind early contrary to
   `AGENTS.md`. If a required E2E cannot run, record it as `Unverified`
   and have the primary session resolve the blocker before claiming red.
6. `implementer` makes those tests green, refactors, reruns the affected
   checks, and self-reviews the diff. Do not weaken the approved oracle;
   revise a contract or test only for a documented reason and reverify it.

### Architecture exploration handoff

Before implementation, preserve the architect's code exploration as a short,
private note in the task's `.tmp/` evidence directory, outside disposable
worktrees. The primary session supplies the absolute destination and records
it in the ledger. `architecte` writes the note there, or returns its contents
for the primary session to save if the assignment is read-only. The primary
session gives `implementer` the absolute path with the contract and failing
test evidence. Pass the same note on resume; do not rely on chat history.

Record only what helps the next worker start or verify the approach:

- **Scope and revision:** task, inspected Git SHA, and any local diff that
  affects the findings.
- **Verified starting points:** repository-relative file paths and symbols
  (lines if useful), what each owns, relevant callers and tests, and the
  evidence behind the observations.
- **Suggested changes:** likely edit points, why they look suitable, a focused
  test or check, and any known gaps or uncertainties. Separate observations
  from guesses; link the agreed contract and any governing ADR separately.

Read the note before rediscovering the code. Verify the relevant pointers in
the implementation checkout and test the suggested approach. The note is a
starting point, not an exhaustive file list or a prescribed implementation.
If the checkout or tests contradict it, choose another approach within the
agreed contract and record what changed and why in the return handoff. Escalate
only when a change would cross an agreed boundary, alter the contract or
approved intent, or require more authorization.

## Review and close

7. For behavior-changing delivery work, request independent reviews from
   `correctness-reviewer` and `test-reviewer` on the exact diff, contract,
   and evidence. Run `security-reviewer` on trust boundaries, access control,
   sensitive data, secrets, and material dependencies; HIGH and CRITICAL
   findings block delivery. Use `rust-perf-reviewer` for ownership,
   concurrency, allocation, and measured performance risks.
8. The author fixes findings, reruns the affected checks, and supplies the
   revised diff. Verify closure rather than carrying a stale PASS forward.
   The primary session records evidence and applies the repository's
   publication, CI, and cleanup rules.

These are roles, not tool restrictions. All eight profiles share the same
tool permissions; reviewers record findings and do not implement the fixes
they review. Use the existing runtime and evidence skills when relevant.
