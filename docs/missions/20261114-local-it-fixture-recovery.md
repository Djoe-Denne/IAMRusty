# Local integration fixture recovery

## Scope

Fix two SDK defects exposed while comparing the unchanged Hive baseline with
local Terra validation. No Terra correction is restored by this change.

The SDK fix is published on RustyCog `main` as
`54af36c22047b995a4f362f3907abeadbc858ec2`. AIForAll consumes that exact commit
through its existing git submodule; the dependency source and CI checkout
strategy are unchanged.

## Findings

The baseline campaign `it-6ac78059d4ad4facb60b2c8224c384ea` passed 69 Hive tests
before the three tests in `roles_api_tests` failed during fixture setup.
The first PostgreSQL start failed on host port 60870. Two subsequent attempts
reused that port and were refused because the failed start had left a container
in `created` state. These are one initial failure and two cascading refusals,
not three independently established name collisions.

`DatabaseConfig::actual_port` used composite cache keys, while
`clear_port_cache` removed the unrelated literal key `db`. The cache is also
used by server, SQS, Kafka, and OpenFGA configuration; clearing the entire map
would invalidate unrelated fixture ports.

`Attempt::start` discarded the typed testcontainers error. The baseline test
log therefore did not explain the first failure. A later read-only inspection
of the preserved container's `State.Error` established a port-allocation conflict
referring to port 60870, not an established Windows permission error. The owner
of the conflicting allocation and the precise timing remain unknown.
Successful Linux CI does not establish successful Windows fixture startup or
recovery after a failed start. The checked-in CI runs Hive integration coverage
on Ubuntu using `cargo llvm-cov`; the local launcher uses native Windows Cargo.

## Changes and invariants

- Namespace database cache keys with `db:` and invalidate only that namespace.
- Preserve stable cached ports between resets, including when multiple database
  configurations are used.
- Classify fixture start failures using typed testcontainers 0.24 errors:
  operation stage, numeric Docker HTTP status, a closed daemon-reason vocabulary,
  or I/O kind and native numeric error code.
- Never format arbitrary error payloads, image descriptors, socket paths,
  container IDs from errors, or daemon messages into the returned diagnostic.
  Unrecognized text remains explicitly unclassified.
- Keep opaque ownership `UNKNOWN`, collision refusal, exact-ID teardown,
  process-exit handling, and the local 24-job Cargo configuration unchanged.
- Do not reserve or atomically allocate host ports as part of this patch.
  Selecting a free host port before Docker binds it still has a race window.
  Cache invalidation prevents reuse of a failed attempt's cached port; it does
  not guarantee that an independently selected replacement cannot collide.

## Validation

Evidence is retained locally under
`.tmp/local-it-fixture-recovery-20261114/`.

- Both new behavioral regressions were demonstrated RED before implementation:
  ineffective database cache reset and missing safe asynchronous start diagnostic.
- Configuration tests: 7 passed.
- Fixture runtime tests: 23 passed, including the actual asynchronous failure
  path, null ownership IDs, secret-sentinel checks, collision protections,
  cancellation, reconciliation, and process-exit handling.
- SDK `cargo fmt --all -- --check`: passed.
- Standard SDK Clippy with `testing`: passed with six existing warnings in
  unchanged command, events, and server-health code. The strict `-D warnings`
  invocation failed at those six locations and is not reported as green.
- Real Hive integration validation: blocked before test execution by local
  compilation errors (`E0463` for Hive crates and dependencies required in
  `rlib` format). No integration test PASS is claimed.

The unit tests execute natively on Windows without Docker. SDK sources are
developed in the sibling repository; the pinned SDK used for Hive is the same
published commit, not an uncommitted dependency override.

## Runtime safety

The existing local Docker Desktop Linux daemon was already running. The Hive
campaign uses the existing `ops/tests/it-native.ps1` ownership and cleanup path,
not a new runner, Compose deployment, or Kind deployment.

Five preexisting containers were inventoried before the campaign: three running
Kind nodes, one exited SMTP fixture, and the previously unknown PostgreSQL
container `edc1e804bd79092f4c624ce2197fe55259868acf0cbb908b2963d06bbf819beb`.
They are not owned by this mission and are not cleanup candidates. Global
Docker Desktop/WSL shutdown is not authorized while those foreign workloads
remain active.

## Integration campaign outcome

The initial nested synchronous capture stopped returning output. A progress
check found no active compiler/test child and no fixture ledger. The owned
stalled Cargo process was stopped, the read-only observer was signalled through
its existing stop marker, and the owned capture process was stopped to release
the blocked pipe. This aborted invocation is not a test result.

The normal native background shell path with direct file logging then completed
campaign `it-76cf06252c4b4896902d40eec2b77b10` with Cargo exit 101 during
compilation. Its final inventory records zero created fixtures, zero owned
containers running, no unknown creation, and no cleanup issue. An independent
full-ID inventory confirms that all five preexisting containers retain their
original states and no new container is present.

Inconsistent or missing linked dependency artifacts are a working hypothesis,
not a demonstrated compiler-cache root cause. A dry run of package-scoped
`cargo clean --profile dev` for RustyCog, Hive configuration, Hive HTTP and Hive
setup selected 6,964 generated files totaling 39.1 GiB. **No cleanup was
executed.** Rebuilding that scope requires an explicit cost/cleanup decision;
there is no global target deletion, job-count reduction, or dependency change
in this mission.

The SDK fixes and their unit evidence are complete. The real Hive integration
gate remains unverified, so the mission is not fully closed.

## References

- `.github/workflows/ci.yml`, `coverage-integration` job.
- `ops/tests/it-native.ps1`, native runner and exact-ID cleanup.
- RustyCog `rustycog-config/src/lib.rs`, database cache resolution/reset.
- RustyCog `rustycog-testing/src/common/fixture_runtime.rs` and
  `fixture_runtime/start_diagnostic.rs`.
- [testcontainers 0.24 typed errors](https://docs.rs/testcontainers/0.24.0/testcontainers/core/error/enum.TestcontainersError.html).
- [GitHub-hosted runner isolation](https://docs.github.com/en/actions/concepts/runners/github-hosted-runners).
