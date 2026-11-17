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
