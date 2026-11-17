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
