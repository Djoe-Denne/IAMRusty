# Telegraph SQS startup and readiness diagnostics

## Scope and evidence

The user authorized a bounded response to two failures in
[CI run 44095213450](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44095213450/job/134084087598):

- The real SQS shutdown oracle timed out waiting for handler entry, before
  requesting shutdown. This does not establish a shutdown implementation defect.
- The password-reset test failed during fixture startup: ListQueues readiness
  exceeded its deadline after 16 attempts, before publishing the business event.

No shared root cause has been established. A previous run also timed out at
handler entry; another intervening Telegraph integration job passed.

## Retained changes

- The real poller test checks its configured region, endpoint and publication
  queue against the live fixture, normalizing only the localhost spelling.
- Startup distinguishes handler entry, parent exit/failure and deadline expiry.
  Every post-spawn assertion follows the existing bounded shutdown/join helper;
  a consumed parent handle is removed rather than polled twice. Existing
  destruction, shutdown-duration and no-acknowledgement assertions remain.
- SDK revision `bfc7b557d7f6a3ac1c5eb0653123a7ab12e62f75` preserves the 30-second
  readiness budget and existing readiness retry/rejection behavior. Errors retain
  closed SDK/HTTP categories, timing, pending-request state and the previous
  completed error. Provider messages, bodies and request URLs are not included.
- On readiness failure, a separate one-second read-only inspection reports only
  the owned container ID, status, running/OOM flags and exit code, or an explicit
  unavailable/timeout observation. No container mutation or automatic retry follows.

## Validation

- SDK readiness tests: 7 passed, 0 failed/ignored.
- Complete SDK library suite (`full,test-utils`): 145 passed, 0 failed/ignored.
- Telegraph startup regressions: 3 passed, 0 failed/ignored; 7 other cases filtered
  for this pure unit selection. These do not establish real SQS readiness.
- Real full Telegraph integration validation via the existing
  `ops/tests/it-native.ps1 -Suite telegraph`: 78 passed, 0 failed/ignored/filtered,
  across 13 reported targets (including an empty target). Both previously failing
  test names passed. This is Windows-native evidence, not a Linux coverage rerun.
- SDK CI [44102475363](https://github.com/Djoe-Denne/rustycog/actions/runs/44102475363)
  completed successfully for the published revision.
- Native evidence: `.tmp/telegraph-sqs-20261117/`.
- Fixture ownership/cleanup evidence:
  `.tmp/itfixtures-it-378bce4155724133b5efb52753c0bc52/`.

## Runtime disposition

The fixture harness recorded 72 owned containers: 63 removed and 9 retained
MailHog containers, all stopped with exit code 2. Zero owned containers remain
running; no uncorrelated new container or unknown creation was reported. These
nonzero-exit artifacts were deliberately retained for diagnosis, not silently
treated as removed or authorized for deletion.

The supplemental creation witness registered 19 new volumes, all still present,
with `safe_to_delete=ask`; no image creation was observed. No volume cleanup,
prune or forced container deletion was performed. The daily resource registry
is `.tmp/docker-resources-20261117.log`. Overall artifact cleanup is therefore
partial, even though the active fixture lease has ended.

All three preexisting running local-full Kind nodes retain their observed
identities and states. Docker Desktop/WSL remain running to preserve those foreign
workloads. No Kind command or global shutdown was used.

The changes make failures observable and preparation cleanup reliable. They are
not evidence that the intermittent CI delivery/readiness cause is resolved.

## Linux CI follow-up and passive probe

[CI run 44104763267, Telegraph job 134118451079](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44104763267/job/134118451079)
still failed handler entry, with the consumer pending and the new configuration
checks passing. Its outbox runtime target reported 9 passed/1 failed. The
password-reset target passed without a ListQueues readiness error in this run.
Another target failed `test_user_email_verified_event_creates_database_notification`
after five SQL polls; it reported 3 passed/1 failed.

The failed notification case successfully published its message and completed
ReceiveMessage calls without an observed handler-level error. That does not
establish where the message went. A successful Windows run does not supersede
these Linux failures.

The user authorized a failure-only passive probe at these two boundaries:

- Request only GetQueueAttributes, with a separate one-second diagnostic limit.
  Do not receive, acknowledge, delete or purge messages.
- Record available, in-flight and delayed counts, visibility timeout and queue
  delay as nonnegative integers. Missing, malformed or overflowing values remain
  unknown; an observation failure never becomes a count of zero.
- Emit a closed `TELEGRAPH_SQS_FAILURE` JSON record with test event/message UUIDs
  and phase/probe timing. Do not emit URLs, arbitrary provider strings, message
  bodies, queue policies, credentials or receipt handles.
- Keep the existing five-second entry wait and notification polling unchanged.
  On the poller failure, sample before teardown, but write the record only after
  the existing shutdown/join helper completes. Do not extend a failed phase or
  turn a later successful diagnostic into a passing test.
- Successful test paths issue no additional broker requests.

These are queue-wide, approximate observations, not proof about an individual
message or its consumer. AWS documents eventual consistency for these counters:
[GetQueueAttributes](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_GetQueueAttributes.html).
The next failing Linux run is still needed to obtain discriminating broker data.

Native probe validation: four HTTP-boundary tests passed in each of the two
affected binaries (eight executions, zero failures/ignored). These cover the
closed numeric projection and identity filtering, missing/invalid counts,
HTTP rejection, an independently bounded stalled request, and the exact passive
API/attribute selection. Other tests were filtered for this focused selection;
no real integration runtime was started or claimed for this follow-up.

## Consumer-local receive boundary follow-up

The passive-probe [run 44117341324](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44117341324/job/134162500404)
failed only the real SQS poller test (13 passed/1 failed in that target). Its
failure record observed zero visible, one in-flight and zero delayed messages,
with visibility timeout 1 second and no queue delay. The phase elapsed time was
5,000 ms and the passive query took 5 ms. These approximate queue-wide counts
do not identify the target message or receiver; they do not establish a parser
failure, acknowledgement or an exclusive LocalStack cause.

The user then authorized a consumer-local observation, implemented in SDK
`55fa6dd61dc3d6ccb27ac288ffd540865e7f3ecc`:

- `test-utils` exposes an opt-in `SqsConsumerProbe`, attached only to the test's
  dedicated consumer and shared with its pollers. It installs no global tracing
  subscriber and makes no extra broker request.
- Counters distinguish receive operations started, succeeded, failed or cancelled,
  returned messages, matches to the expected SQS message ID, absent IDs/bodies,
  parse failures, unsupported event types and handler invocation boundaries.
  Operation counters do not count individual internal HTTP retry attempts.
- A small RAII witness counts cancellation while awaiting the receive operation.
  Cancelling an already-entered suspended handler does not count as a cancelled
  receive. None of these observations acknowledge or change a message.
- Failure JSON samples the counters at the original phase failure, after the
  existing passive broker probe, and after the existing shutdown/join path.
  The snapshot contains counters only, not the stored expected message ID,
  payload, receipt handle, credentials or provider messages.
- The live samples are cumulative and non-transactional across concurrent pollers.
  Handler invocation counters are distinct from the test handler's notification;
  they mark the call boundary, not a separate assertion that its body completed.
  All acceptance deadlines, construction order, cleanup and no-ack oracles remain.

Validation: six new observer HTTP-boundary/unit cases, including the public
start/stop wiring, passed within the complete SDK library suite of 151 tests
(zero failed/ignored/filtered). The SDK also passes a native library check with
only `events` and without `test-utils`. The consumed pin compiles Telegraph's
outbox runtime target; its three startup regressions pass (11 other cases
filtered). These are unit/compile results, not a new real-broker run or a claim
that the intermittent CI failure is corrected. No new Docker runtime lease or
resource creation was needed for this instrumentation.

## Terminal cached fixture correction and targeted CI

The next full CI run, [44136915301](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44136915301/job/134233387768),
passed the poller, password-reset and email-verification scenarios. It emitted no
consumer failure record. Instead, 24 setups failed across two notification test
binaries, repeatedly waiting 30 seconds against their respective exited
LocalStack containers. Both reported exit code 245 and `OOMKilled=false`.
Neither value establishes the initial cause.

SDK `feaabaf7ddf3b9000b43ec3f5050d27528e71171` corrects the repeated waits:

- Before SQS protocol preparation, inspect the exact owned container ID within
  one second. Only confirmed exited/dead, non-running states or confirmed absence
  are terminal. Unknown and transitional states still require protocol readiness.
- Preserve the first terminal observation in the existing fixture owner.
  Subsequent setups return that error without another inspect, log request or
  protocol wait. Do not recreate the container or change ownership/cleanup.
- On the first terminal observation, sample up to 100 log lines, 16 KiB and 64
  frames within one second. Emit only fixed marker booleans and numeric sample
  sizes, alongside the closed container-state projection. Never emit raw logs,
  provider text, inspect health/error fields or credentials. Markers are hints
  about sampled text, not proof of a root cause. Cancellation retains the
  terminal identity even if the log sample cannot finish.
- Keep all existing readiness budgets and test acceptance assertions unchanged.

The complete SDK library suite passes 157 tests, including six new regressions
for terminal classification, bounded private-data-free log projection, retained
first failure, unknown/running states and cancellation. This is unit evidence;
the production Docker inspect/log wiring still requires the Linux run.
The consumed SDK pin also compiles Telegraph's outbox runtime binary; its three
startup regressions pass (11 other tests filtered). Final temporary-workflow
YAML, Bash and embedded Python syntax checks pass.

The user authorized a temporary manual workflow,
`.github/workflows/telegraph-sqs-diagnostic.yml`, selecting only
`notification_error_scenarios_test`, `notification_http_endpoints_test` and
`outbox_runtime_tests`. It uses the existing pinned setup/cache actions,
`cargo-llvm-cov 0.9.1` and explicit Ubuntu 26.04 (the failing runner family), with
no dependency on unrelated build jobs. Artifacts contain closed diagnostic
records, source/runner metadata and fixture ownership ledgers, not raw test or
container logs. A separate exact-ID check stops any remaining owned containers
without deleting or adopting pre-existing resources. The workflow must be
removed after the investigation's targeted validation.

No local Docker/Kind/WSL runtime was started for this correction. Historical
retained resources and the sibling SDK's unrelated untracked `nul` are unchanged.

### First targeted run

[Run 44146991942](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44146991942/job/134264737957)
executed all 48 selected tests successfully (14 error-scenario tests, 20 HTTP
endpoint tests and 14 outbox runtime tests; zero failed/ignored/filtered).
Elapsed job time was approximately six minutes rather than waiting for the full
pipeline. No terminal fixture or consumer failure record was produced, so this
run does not explain the historical exit 245 or poller intermittence.

The overall job nevertheless failed at the supplemental Docker verification,
after tests had passed. The retained ledgers contain 301 events covering 38
owned container IDs: 26 removed and 12 confirmed stopped by the SDK's process
exit path. The independent final Docker check was inconclusive; its generic
inspect error was not retained, and no cause for that CLI failure is asserted.

The workflow-only correction replaces error-string matching with successful
full-ID inventories, exact owned-ID stops if needed, and a final running-ID
comparison. Five isolated contract cases pass (removed, already stopped,
running owned, pre-existing refusal and daemon failure), without real Docker
commands. Application, SDK and test sources are unchanged for the next targeted
run; it validates the corrected evidence/cleanup path, not a blind attempt to
turn an intermittent test green.

### SDK inclusion seam

The SDK's independent CI identified an additional module-resolution boundary:
`tests/fixture_runtime.rs` includes the runtime source through `#[path]`.
The new observation submodule needs the same explicit relative path as the
existing runtime submodules. The full SDK format check passes after that fix.

Executing this previously uncovered test target then exposed an existing
hard-coded child-test filter: it named the library inclusion only and silently
selected zero tests in the integration inclusion. The correction derives the
exact libtest name from the current module path. It preserves the assertion
that exactly one child fixture runs and all existing exit-code, ledger and
failure assertions; no lifecycle acceptance oracle is weakened.

SDK `1db6c9de8708f6451d91a56f482b053026487c8f` publishes these two inclusion fixes.
Native validation passes all 28 fixture-runtime target tests and the same
process-exit contract in the library inclusion (one selected, 156 filtered);
the final complete SDK formatting check passes. These follow-up changes are
module resolution and test selection only, not a new SQS behavior.
The complete SDK [CI run 44149650275](https://github.com/Djoe-Denne/rustycog/actions/runs/44149650275)
also succeeds for that exact revision. Final root workspace formatting passes.

### Final targeted result and workflow removal

[Run 44148703762](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44148703762/job/134270769702)
passes the entire targeted job in 6 minutes 20 seconds. All 48 selected tests
pass (14 + 20 + 14, zero failed/ignored/filtered), including the real SQS poller.
The test exit code is zero and the closed diagnostic record list is empty.

The independent full-ID check covers all 38 identities in the fixture ledgers:
26 are absent after removal, 12 are stopped, none are running, and no additional
stop was needed. This agrees with the ledgers' 26 removal events and 12 verified
process-exit stops. Artifact `telegraph-sqs-diagnostic-44148703762` preserves the
closed diagnostic/ownership snapshots and source metadata. The tested root is
`41f42076460534beb1e4ff74394af2315457fc7b`, SDK `feaabaf`; the subsequent SDK
inclusion-only fix is validated separately above, not represented as part of
that Linux integration run.

The temporary workflow is removed after this validation, as requested. No
supporting tracked script, mission branch or worktree was created. Local logs
and downloaded evidence remain ignored under `.tmp/telegraph-sqs-20261117/`.
No local Docker runtime was started or cleaned, and historical retained
resources were not adopted.
The SDK's pre-existing `mission-telegraph-ac7-zlfadf` branch predates this
follow-up and is already an ancestor of `main`; it is preserved, not treated as
a newly created mission branch.

The repeated-wait defect now has a fail-fast correction and bounded first-exit
diagnostics. Neither successful targeted run reproduces the historical exit
245 or receive/handler failure, so their original causes remain unresolved.
These results are not a claim that intermittence is eliminated or that the
full workspace integration pipeline passed.
