# Telegraph IT: require SQS protocol readiness

## Approved scope and observed failure

The user approved extending the completed Manifesto repair to the remaining
Telegraph failure in CI 43569896393, job 132373537827. Manifesto build, unit tests,
integration tests and E2E were green in that run.

`outbox_runtime_tests` had 2 passes and 5 failures during fixture setup. The first
failure was `Failed to create test SQS: dispatch failure`; later fixture attempts
exhausted TCP readiness. No failed business assertion was established.

The unchanged seven-case binary passed locally. This does not invalidate the
intermittent CI failure or prove its precise first-request transport cause.

## Reproduced contract defect

The shared SQS fixture previously accepted a TCP connection, slept two seconds,
then attempted `CreateQueue`. An open listener did not prove that SQS was ready.

A regression against an HTTP 503 listener failed **before** the readiness logic
was changed: the old helper incorrectly returned success after 2.01 seconds.
This is a deterministic readiness-contract failure, not a claim that a mocked
HTTP service reproduces all LocalStack startup behavior.

## Bounded SDK correction

- Require an actual SDK `ListQueues` success before creating configured queues.
- Use one 30-second readiness deadline, including an in-flight SDK request.
- Permit transient transport/HTTP 5xx/429 readiness recovery within that budget.
- Reject permanent HTTP errors or malformed successful responses.
- Remove the fixed two-second warmup sleep.
- Do not retry business operations, restart the test, modify authentication,
  change public APIs, alter port/configuration caches or change container cleanup.

The private wait helper has one caller, `TestSqs::new`, verified through Serena.
SDK development is in the sibling repository; the canonical index does not cover
its uncommitted changes, so edits use bounded source patches.

Five unit tests cover persistent 503, 503→429→valid SQS recovery, permanent 403,
invalid 200 response and a stalled request. Their test client disables the SDK's
own retries, proving the fixture's readiness behavior rather than an implicit
transport retry. They assert `ListQueues` requests, not authentication or real
LocalStack behavior.

## Verified local results

Windows native, one Cargo at a time, 24 configured jobs, target E:

```text
powershell.exe -NoProfile -File ops/tests/it-native.ps1 -Suite telegraph -TestBinary outbox_runtime_tests
cargo test --locked --manifest-path ../rustycog/Cargo.toml --features full,testing --lib
cargo clippy --locked --manifest-path ../rustycog/Cargo.toml --features full,testing --lib --tests --message-format=json
cargo fmt --manifest-path ../rustycog/Cargo.toml --all -- --check
```

- Unchanged real outbox baseline: **7/7**, zero ignored/filtered, 103.45 seconds.
- New readiness tests: **5/5**, with the old behavior first observed red.
- Full SDK library suite: **137/137**, zero ignored/filtered; Kafka is not enabled
  for these native Windows checks.
- SDK formatting and Clippy pass, with no diagnostic attached to changed SQS files.
- Corrective pinned outbox binary: **7/7**, zero ignored/filtered, 92.26 seconds,
  using the same real infrastructure and unchanged assertions.
- The full AIForAll CI still needs its exact-revision result; no global success
  is claimed at this stage.

## SDK publication and CI

SDK commit **41d6d7bd3f9939839e1c044f084fc6102d2bfc85** is published on
`origin/main`, with the exact remote reference verified before pinning AIForAll.
Its [CI 43574365173](https://github.com/Djoe-Denne/rustycog/actions/runs/43574365173)
completed **successfully**: Format, Unit Tests And Coverage, Clippy and SonarCloud.

Evidence directory: `E:/cargo-target/AIForAll/telegraph-ci-repair-20261114/`.
CI log: `.tmp/telegraph-ci-43569896393.log`.

## Runtime ownership

Baseline campaign `it-b1d4e77557b2415aa4e15f36db1c4d0a` created 8 containers,
removed 7 and left **zero owned running containers**. One task-owned MailHog
container exited with code 2 and was initially retained:
`b10e79d103d5153ad2f046f803bc7553f6e169498e7c32946fc7c019339a3d2d`.
The corrective campaign `it-3748088061854bb08d370ca07a28bad5` also created 8,
removed 7 and left zero running; its initially retained MailHog ID was
`75bdb4505b12cca9166d305ceeee100cb764b2700ec2e476f89f32bb9056604c`.
Neither was OOM-killed. These stopped SMTP resources are not evidence of the CI
SQS cause.

After explicit user authorization, both stopped MailHog containers were removed
by exact ID, without force, volume/image deletion or prune. Logs and state were
archived with SHA256 manifest under
`E:/cargo-target/AIForAll/telegraph-ci-repair-20261114/smtp-cleanup/`.
All daily registries were reconciled through the existing lifecycle helper;
193 unrelated historical malformed rows were reported and left untouched.
No exact-ID ambiguity or foreign lease applied to these two targets.

Final inventory: **16 task containers created / 16 removed / zero running**,
and the original four foreign IDs and states are unchanged. No new unowned
container remains.

Four preexisting resources remain outside scope: three running full-profile Kind
nodes and one stopped SMTP fixture. No Kind, Desktop or WSL startup/shutdown was
performed. The existing it-native lifecycle records exact IDs in
`.tmp/docker-resources-20261114.log`; no ownership is inferred from names.

## Sources and limits

- [LocalStack SQS documentation](https://docs.localstack.cloud/aws/services/sqs/)
  documents the `ListQueues` operation.
- [Pinned LocalStack 3.0.2 configuration source](https://raw.githubusercontent.com/localstack/localstack/v3.0.2/localstack/config.py)
  makes eager service loading opt-in. Neither source proves this CI instance's
  exact transport failure.
- A historical wiki entry recorded similar initial `CreateQueue` failures and an
  inferred flake. Existing Manifesto setup retries were inspected but not copied
  or expanded as part of this correction.

The observed false-readiness condition is corrected and regression-tested.
The real pinned IT passes. Whether this fully resolves the original intermittent
CI setup failure remains a question for the exact-revision CI check.

Continuity: `.cursor/handoffs/20261114-telegraph-it-ci-repair.md`.
Verification contract: `.cursor/qa-plans/20261114-telegraph-it-ci-repair.md`.
