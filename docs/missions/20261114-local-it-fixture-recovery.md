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

## 2026-11-15 observed retry and authorized pagefile adaptation

The user requested another unchanged compilation after freeing host resources.
The available physical memory was approximately 42 GB, but the idle system
commit headroom was only about 21 GB. The ordinary 24-job build again failed.
Windows logged Resource-Exhaustion-Detector Event 2004 during that invocation.

The user then requested a reproduction while personally monitoring memory.
The session-owned one-second sampler captured 15 rustc processes with
33,380,675,584 aggregate private bytes, but only 1,616,265,216 resident working
bytes. Substantial free physical RAM therefore did not establish adequate commit
capacity. This log directly reported dependency metadata memory-mapping
failures with Windows error 1455: six E0786 diagnostics, 17 E0463 diagnostics,
and three explicit paging-file-insufficient errors. No corrupt artifact bytes
were established and no further clean was performed.

The user explicitly authorized adapting Windows paging capacity. An
administrator preflight found one existing paging file, `C:\pagefile.sys`,
configured with an 8,192 MiB initial size and 32,768 MiB maximum, with global
automatic management disabled. C: was NTFS with approximately 147 GB free.

Only that entry was updated:

- Initial size: **65,536 MiB (64 GiB)**.
- Maximum size: **98,304 MiB (96 GiB)**.
- Automatic management remains disabled, as before.
- The maximum growth still leaves approximately 49 GiB free based on the
  preflight; the change script enforced a 20 GiB disk safety margin.

The setting was read back successfully. At this interim checkpoint it was not
active: the runtime paging allocation remained 8,192 MiB and the observed
commit limit remained 74,684,157,952 bytes. A restart would have been required
to validate that C: resizing. The subsequent I: configuration below supersedes
this interim state. No restart, job reduction, profile change, shared-process
termination, or Docker/WSL operation was performed.

The prechange snapshot and read-back are retained as `pagefile-before.json`
and `pagefile-after.json` under
`.tmp/local-it-fixture-recovery-20261115/`. The previous values were
8,192/32,768 MiB; restoring them would be a separate authorized action.
This paging change is not an integration-test PASS.

### Final disk preference: I:

The user selected I: instead of the proposed E:. Read-only preflight identified
a healthy, writable local NVMe volume with NTFS, no BitLocker encryption and
approximately 694 GiB free. After guarding the exact interim configuration,
the new I: entry was created and verified before restoring C: to its original
sizes:

- `I:\pagefile.sys`: **65,536 MiB initial / 98,304 MiB maximum**.
- `C:\pagefile.sys`: **8,192 MiB initial / 32,768 MiB maximum**.
- Global automatic management remains disabled.
- No pagefile was manually moved or deleted; no restart was performed.

Unlike the interim C: resize, the I: pagefile became active immediately.
`Win32_PageFileUsage` reports 65,536 MiB allocated on I: and 8,192 MiB on C:.
The measured commit limit increased to **143,403,634,688 bytes**, exactly
64 GiB above its prior value. Therefore a reboot is **not needed** to use this
verified capacity. I: retains approximately 630 GiB free after allocation.

Snapshots are `pagefile-I-before.json`, `pagefile-I-encryption.json`, and
`pagefile-I-after.json` in the 2026-11-15 evidence directory. The actual runtime
read-back, not the startup setting alone, authorized a new unchanged full build
at 24 jobs, without cleaning.

### Full build after the I: change: PASS

`cargo build --workspace --locked --all-targets` completed with exit 0 on
2026-11-15, 09:40:50–09:53:11 UTC. Cargo reported 12 minutes 19 seconds with the
ordinary dev profile and 24 configured jobs. Executed sources were
`8379f5cd5869ab7190c9c5c4585f255f9a698386` and SDK
`54af36c22047b995a4f362f3907abeadbc858ec2`; application/config/test sources
were verified unchanged from the earlier build apart from this report.

The 550 memory samples recorded peak system commit of
142,991,785,984 bytes against a 143,403,634,688-byte limit. The maximum
aggregate rustc private allocation was approximately 82.17 GB, with a working
set of approximately 10.22 GB. The earlier smaller commit limits could not
accommodate that demand even with considerable free physical RAM.

Evidence is `pagefile-i-build/build-result.json`, `build.log`, `memory.jsonl`
and derived `memory-summary.json` in the 2026-11-15 evidence directory.
The invocation-owned sampler was disposed. No clean, concurrency exception,
profile change, shared-process shutdown, or compiler change was used.

The ensuing workspace library execution failed in the legacy GitLab fixture
`consumed_verifier_reaches_vendor_token_post`, not in compilation. Wiremock's
`set_body_string` overwrote its JSON MIME type with `text/plain`; an isolated
prebuilt execution reproduced the failure. Before that tiny fixture-only fix
was delivered, the user explicitly required retirement of both standalone
vendor services and their tests, since generic OAuthConnect replaces them.

### Repository retirement supersedes the legacy fixture repair

The [retirement contract](../contracts/legacy-connect-retirement.md) reconciles
that instruction with ADR0413, without claiming live cutover or real-provider
identity equivalence. All 60 tracked legacy files and 12 packages are removed;
generic OAuthConnect, shared IAM/Connect security tests, public provider IDs,
callback URLs and existing credential paths remain. No SDK bump or dependency
upgrade accompanies the reduced lockfile. Locked offline workspace metadata
reports 59 members and all six OAuthConnect packages.

Active image/chart/Compose/PKI bindings and IAM default URLs are updated to the
single service. Operator-owned Compose configuration is required; no credentials
or nonexistent service configuration directory are fabricated. Current guidance
is [OAuthConnect](../services/oauth-connect.md); old handbooks are archived.

The pre-retirement build PASS above does **not** validate this source delta.
The first reduced-workspace native campaign passed under
`.tmp/retire-legacy-connect-20261115/native/`: full all-target build,
688 workspace-library tests, 31 shared-contract tests with `server`, and
267 generic Connect tests; zero failures, ignored or filtered tests.
These phase totals overlap and are not a unique-test sum.

After that invocation released Cargo, the final small IAM probe-exemption
cleanup was applied (exactly the two IAM probe paths remain), together with
removal of the unused workspace OAuth2 4.4 declaration and an instance-URL
rustdoc correction. All four native gates **passed again on that final source**
under `native-final/`, 10:49:39–10:56:01 UTC: full build, 688 workspace-library,
31 shared-contract and 267 generic tests, each zero failed/ignored/filtered.
No source edit occurred while Cargo was active.

Final offline retirement/build/simulator/infra checks passed (32 tests), as did quiet
Compose model validation without a daemon. The broader 51-test static run is
RED (14 failures and one error, including subtests), involving native PKI ACL
ownership and Bao CLI private-file handling in this elevated environment.
Two isolated probes against pristine published HEAD reproduce those failures;
neither the tests nor private-file protections are weakened to obtain GREEN.
The entire broader run is not reported as PASS.
The independent infra verifier confirmed offline binding coherence and preserved
credential paths; live readiness remains NOT RUN/INCONCLUSIVE. Helm rendering
is NOT RUN because the native Helm CLI is unavailable; it was not installed.

### IT startup blocked; mission not fully closed

The final host preflight found no Cargo, Docker Desktop/backend or WSL runtime
process. Available RAM was 51,026 MB and the commit limit remained
143,403,634,688 bytes. The requested IT startup was scoped to existing
testcontainers fixtures, with no Kind or application-container launch.

`docker desktop start --timeout 180` failed with “Docker Desktop is still
starting: context deadline exceeded.” A separate bounded diagnostic found no
engine named pipe and no Desktop/backend/vmmemWSL process. No repeat startup or
runtime repair was attempted. The five-suite IT wrapper was **not invoked**:
**zero IT tests executed**, zero fixture resources created, and no new Docker
resource-registry entry. Existing Docker data was not inspected, adopted or
removed. Runtime repair requires the separately specified user authorization.

Read-only WSL inventory subsequently showed `docker-desktop`, `AIForAll` and
`ARTIST-Ubuntu` all stopped. No shutdown, process kill, deletion or prune was
needed. The runtime lease is released; the startup log and runtime observations
remain under `.tmp/retire-legacy-connect-20261115/`.

The publication audit also kept an ignored, preexisting root `.dockerignore`
out of the deliverable. Instead, the tracked OAuthConnect runtime-specific
ignore excludes its entire build context, and default operator directories are
outside the repository (`../aiforall-oauth-connect/{config,runtime}`). The
artifact builder excludes the optional ignored `.local/oauth-connect` location.
These ignore/configuration-only refinements leave the validated native Rust
inputs unchanged; all 32 static checks and quiet offline Compose validation
passed again on the publication candidate.

The retirement deliverables can be preserved and published, but the original
full test campaign is **not complete**. Resume after authorized Docker runtime
recovery, inventory the daemon before fixtures, and run the five existing native
IT suites with their exact-ID lifecycle controls. Preserve the documented
baseline static failures and unavailable Helm check; neither is converted to PASS.

### User-restarted Docker: IT resumed

The user subsequently reported a Docker crash and manually restarted Desktop.
The expected local named-pipe engine then responded (29.8.2), clearing the
startup blocker without agent-performed runtime repair.

Fresh inventory before fixtures found five preexisting containers: three running
local-full nodes, one created historical Postgres fixture, and one exited
historical SMTP fixture; 99 images and 346 volumes were also present. They remain
foreign to this campaign and must not be adopted, restarted, stopped or removed.
The user-started Desktop is not campaign-owned and must not be globally stopped
while that existing lab runs.

The same guarded five-suite native IT campaign is now launched. Container
creation/cleanup uses the unchanged runner; a temporary image/volume event
witness supplements its container registry, using daemon time, exact Engine IDs
and a shared append lock. Such observations alone do not confer ownership.
All required active Windows fixture images were already cached; no delivery
build or Kind operation was performed.

### Resumed IT result: Hive RED; later suites not run

The campaign stopped in Hive at 11:14:59 UTC: **58 tests passed and one failed**,
with zero ignored tests across ten completed test binaries. The test
`create_two_organizations_preserves_scoped_default_role_permissions` failed
during `setup_test_server`, before its organization assertions.
Manifesto, Lazaret, IAM and Telegraph were not started after this failure.

The Engine's recorded start error is a PostgreSQL fixture binding
`0.0.0.0:60884/tcp`: **address already in use**. A subsequent read-only query
found no matching Windows excluded range. The SDK's random-port path binds
`127.0.0.1:0`, reads the selected port, releases that socket and later requests
the fixed Docker mapping; a reservation gap exists. The historical competing
owner was not captured, so this work does not claim an exclusive root cause.
A later fixture in that binary did start on a different port. No retry,
concurrency reduction, bind-range change, unrelated process stop or incidental
SDK bump was used to turn the failure green.

The existing harness removed **14 joined/owned containers**, leaving **zero
owned containers running**. One Engine-created container has no matching
fixture creation ledger and remains in `Created` state:
`d7e48cd7a9f4ba72b150b3117c12f0d22fee9756ff70625268d84683bb643453`.
It is `ask`, not adopted or deleted. The five preexisting containers remain
present with unchanged states. Cleanup is **INCONCLUSIVE**, not complete.

The supplemental witness recorded eight anonymous volumes and no new image
creation events. None of those volumes was observed destroyed; exact volume
Names and mount correlations are retained in `integration/extra-events.jsonl`
and `.tmp/docker-resources-20261115.log`, initially classified `ask`. No volume
or unknown container was manually removed. Cargo and both observers exited.
The user-started Desktop and preexisting lab remain untouched.

Evidence: `.tmp/retire-legacy-connect-20261115/integration/` and
`.tmp/itfixtures-it-5e1c9f5b39264ed5a05331683e51398a/`, with the bounded
Engine/host observation in `hive-port-start-error.json`.
The source-retirement deliverable is natively validated, but the full IT
campaign is **RED/incomplete**. Resolve the fixture allocation/opaque-creation
boundary in a separately scoped SDK/harness change and arbitrate retained
resources before resuming; do not bypass the cleanup hold via another runner.

### Experimental fixture rewrite withdrawn at user request

The later PostgreSQL allocation/ownership rewrite did not establish a stable
integration campaign. The user requested its complete withdrawal and a fresh
investigation rather than further patches. Both experimental SDK commits were
reversed by published commit `0657936f7a77f8abdc0210bfd6aa1ba7de897428`.
Its complete source tree is identical to the retained pre-experiment SDK
`54af36c22047b995a4f362f3907abeadbc858ec2`; AIForAll consumes that original pin
again. No new fixture backend, mapped-port publication API or experimental
regression remains. The experimental implementation contract and temporary
publication/test launchers were removed; historical logs remain evidence only.

The retained work is the standalone Connect retirement, generic IAM path
alignment and the pre-experiment SDK diagnostics/cache correction. Full native
gates had passed on that baseline before the later IAM test-path edits. Those
edits retain their original HMAC, receipt and confidentiality assertions; the
latest confidentiality-test path edit has not completed a successful live run.
No entire IT campaign is claimed green.

Historical experimental runs reached Hive78/0 and IAM221/1. After a local test
matcher correction, the next IAM run stopped earlier at92passed/1failed:
fixture setup refused a name/port collision. Its creation inventory contains
an exited container with the same generated name as the refused attempt;
separate process-incarnation ledgers share PID45160 and attempt0. This is a
verified observation, not a complete explanation of all earlier failures.
The eight owned containers from that final run were cleaned, with zero owned
running and no new unknown creation. The later three suites were not started.

Evidence remains in `.tmp/postgres-daemon-port-20261115/` and
`.tmp/retire-legacy-connect-20261115/integration/`. Do not rerun removed
experimental launchers or reinstate their design as an accepted solution.
No build/test was rerun for withdrawal; exact source-tree equivalence and Git
diff checks establish removal, not resolution of the functional fixture issue.

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
- [Windows pagefile settings versus runtime usage](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-pagefilesetting).
- [Allocation failures caused by slow pagefile growth](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/slow-page-file-growth-memory-allocation-errors).
