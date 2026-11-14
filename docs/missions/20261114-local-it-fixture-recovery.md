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

At the initial checkpoint, inconsistent or missing linked dependency artifacts
were a working hypothesis, not a demonstrated compiler-cache root cause. A dry run of package-scoped
`cargo clean --profile dev` for RustyCog, Hive configuration, Hive HTTP and Hive
setup selected 6,964 generated files totaling 39.1 GiB. No cleanup had been
executed at that checkpoint. The later user-authorized full clean is recorded
below; it does not authorize Docker cleanup or changes to the job count.

The SDK fixes and their unit evidence are complete. The real Hive integration
gate remains unverified, so the mission is not fully closed.

## Authorized overnight clean and cold-build failure

The user subsequently authorized a full Cargo clean, build, and test campaign.
The target was verified by Cargo metadata as `E:\cargo-target\AIForAll`, with
71 workspace packages and 217 integration-test targets. The cold-build command
retained the existing default profile and 24 configured Cargo jobs.

Two preflights were preserved without executing a clean: Docker was unavailable,
and an unrelated Cargo process later held the native slot. Docker was decoupled
from compilation; the unrelated Cargo processes were awaited rather than
interrupted.

The full clean then succeeded, removing 178,744 generated files totaling
351.3 GiB. The cold `cargo build --workspace --locked --all-targets` failed
before tests, with the first causal failures being refused 1–2 MiB allocations
and an archive/object mapping error with Windows error 1455. The E0463 errors
followed these allocation failures; LLVM also explicitly reported out of memory.
Four compiler processes terminated with fail-fast status `0xc0000409`, which
does not by itself establish a stack overflow or compiler defect.

Windows Resource-Exhaustion-Detector System Event 2004 at
`2026-11-14T21:56:32.2038959Z` independently confirms virtual-memory exhaustion
at the time of the crashes. It lists approximately 27.05 GB committed by WSL,
18.75 GB by the shared Rust code indexer, and 4.04 GB by a Rust language server.
Those foreign/shared workloads were not stopped or unloaded. The postfailure
commit snapshot was approximately 82.4 GB against a 100.45 GB limit; physical
free RAM alone is not a sufficient build budget.

This establishes the cause of the **cold-build** failure. The earlier nonclean
Hive E0463 log lacks an explicit allocation failure and is not retrospectively
assigned the same proven cause.

Evidence: `night-cold-3/clean.log`, `build.log`, `status.json`,
`host-resources.json`, and `compiler-crash-events.json` below the mission's
local evidence directory. No unit or integration PASS is claimed for this cold
attempt.

## Bounded staged recovery

An escalated read-only review independently identified the first causal errors
and recommended one measured recovery experiment:

1. Build workspace libraries with `cargo build --workspace --locked --lib`.
2. Build complete existing service families with `--all-targets` and Cargo
   package globs, retaining 24 configured jobs.
3. Require the original `cargo build --workspace --locked --all-targets` gate.
4. Run workspace library tests and the five existing integration suite runners:
   Hive, Manifesto, Lazaret, IAM, and Telegraph.

Staging is not acceptance. Resolver feature unification may produce additional
variants, so a successful staged build cannot replace the full-workspace gate.
A failed phase stops the experiment rather than triggering another clean or
indefinite subdivision. Nonzero unit/IT exits are not assumed to be ordinary
assertion failures: compilation, memory exhaustion, or failed cleanup must not
silently permit further suites to start.

The temporary native runner writes direct-to-file logs and records host
commit/limit samples through an invocation-owned measurement job, stopped in
its `finally` block. There is no pagefile, toolchain, debug profile, global
configuration, job-count, assertion, MCP-residency, or WSL change. Docker starts
on demand only after compilation and unit execution, for the existing real
testcontainer fixtures.

### Recovery result

- Workspace libraries: **PASS**, exit 0.
- Hive family, all targets: **FAIL**, exit 101. A refused 2 MiB allocation
  precedes the E0463 and compiler fail-fast cascades.
- Other service families, final workspace acceptance, unit execution, and all
  five IT suites: **NOT RUN**.

The 20 host samples started at approximately 83.52 GB committed against a
100.45 GB limit. The maximum sampled commit was 93.73 GB; five-second sampling
cannot exclude a higher intervening peak. Sampling completed without a
measurement error, and the invocation-owned measurement job was stopped and
removed.

The experiment stopped at its first renewed allocation failure. No additional
clean, target subdivision, build retry, changed profile, or weakened gate was
performed. A static review also identified unsafe continuation after
unclassified unit or suite failures in the temporary wrappers. Those wrappers
were made conservative: every nonzero unit or IT exit stops later phases.
Neither corrected branch had been reached in this experiment.

The overnight campaign created no Docker resource and did not start Docker
Desktop. Final non-waking process observation found no owned Cargo or campaign
wrapper and no Desktop/backend process. The preexisting WSL workload remained
active and was not shut down. No claim is made about daemon-container state
while the daemon is unavailable.

Evidence is under `night-staged-1/`; the allow-listed
`night-evidence-manifest.json` records sizes and SHA-256 hashes of the ten
relevant evidence files. The earlier 30 SDK targeted tests remain historical
evidence for the unchanged SDK fix, not an overnight full-validation PASS.

**Resume condition:** a verified, authorized increase in available commit
capacity, or explicit arbitration of the fixed 24-job/resource constraints.
No evidence-backed compilation recovery remains within the current resource
limits. The mission remains functionally blocked, not green.

## References

- `.github/workflows/ci.yml`, `coverage-integration` job.
- `ops/tests/it-native.ps1`, native runner and exact-ID cleanup.
- RustyCog `rustycog-config/src/lib.rs`, database cache resolution/reset.
- RustyCog `rustycog-testing/src/common/fixture_runtime.rs` and
  `fixture_runtime/start_diagnostic.rs`.
- [testcontainers 0.24 typed errors](https://docs.rs/testcontainers/0.24.0/testcontainers/core/error/enum.TestcontainersError.html).
- [GitHub-hosted runner isolation](https://docs.github.com/en/actions/concepts/runners/github-hosted-runners).
- [Windows page files and the commit limit](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/introduction-to-the-page-file).
- [Windows fail-fast status is not necessarily a stack overrun](https://devblogs.microsoft.com/oldnewthing/20190108-00/?p=100655).
- [Cargo package selection](https://doc.rust-lang.org/cargo/commands/cargo-build.html#package-selection).
- [Cargo feature unification](https://doc.rust-lang.org/cargo/reference/resolver.html#features).
