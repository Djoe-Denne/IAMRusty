# Isolated IAM compile-once pilot

**Status: Proposed.** Forward amendment to [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md), authored with `architecte`.

The reserved workflow `.github/workflows/iam-compile-once.yml` runs only on the trusted `ci-iam-compile-once` push branch. It builds the ordinary IAM server and migration executables and the exact existing IAM test assignment with coverage instrumentation on one Ubuntu 24.04 x64 producer. Compatible Cargo units may be shared; distinct profiles, features, hosts, test harnesses or build-script inputs remain distinct. An existing `test-mode` or `qa-mode` feature is not enabled.

The producer selects executable paths from Cargo JSON, not from a directory scan. The test shards fetch only their own bounded executable bundles; they keep the existing package working directories, libtest defaults, fixture setup and ownership. Consumers run no application compilation. The pilot must verify direct libtest replay on the hosted runner, including relocation, package environment, dynamic libraries, generated assets and target conservation; static tests alone do not establish runtime parity.

Artifacts have experimental schemas and names. Consumers verify the repository, workflow, branch, checkout, SDK, run and attempt, successful producer, API digest, member list and executable hashes. OAuth and ext-authz remain native; only IAM may be instrumented. Canonical native and saved-image admission do not accept these bundles. The pilot checks image ABI and cleanup before the unchanged five-phase harness runs. IAM profiles from that campaign stay private and out of coverage. Shard LCOV is experimental only; the pilot cannot feed Sonar, Pages, release, or canonical coverage.

Compare producer Cargo time, variant evidence, artifact bytes, job and transfer durations, actual test and E2E outcomes, and cleanup versus a comparable same-source baseline. Report wait time and the whole critical path, including the new wait for test executable production. Do not call a successful producer or static validation an end-to-end speedup. No default CI scheduling, Docker fixture, app, ABI policy, provider, or cache policy changes are part of this pilot.

### Compact binaries and earlier E2E availability

Set both `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` to `line-tables-only` exclusively in the IAM producer. This retains source locations for backtraces but removes fuller debugging information and changes compilation/cache inputs. Hold the existing executable and bundle limits and coverage instrumentation fixed. A successful replay must export actual covered IAM source lines from the compact binaries; assess mapping and outcome parity before adoption.

Build, archive and upload the instrumented server and migration before starting test compilation in the same target directory. This freezes the E2E payload before Cargo can overwrite output paths. Record build completion, archive bytes, upload completion, test compilation, visible fresh/changed Cargo records, and consumer start. `fresh` measures Cargo reuse, not sccache hits. GitHub `needs` and the existing successful-producer admission still prevent E2E execution before the entire job succeeds; the early upload is an availability measurement, not a scheduling change.

### Dedicated hosted E2E gate

The reserved workflow now qualifies E2E execution only. It compiles and uploads the instrumented IAM server and migration, builds the existing native companions, then runs the unchanged hosted k6 campaign without compiling in the consumer. Test-executable production and replay stay out of this follow-up so they do not delay the gate. The earlier pilot exercised those partitions independently of the E2E outcome. The qualification must show the exact same-attempt binaries, successful health, OAuth browser and TLS/SAN scenarios, and verified cleanup; it does not authorize substituting these binaries in release validation.

### Qualified gate and bounded adoption

Run [44493607389, attempt 1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44493607389)
passed the five E2E scenarios, restoration and cleanup. The user's conditional
main-CI rollout authorization is effective under the separate
[IAM-only adoption contract](ci-iam-compile-once-adoption.md). This pilot
envelope and its evidence remain experimental; no canonical or release
admission is retroactively granted.
