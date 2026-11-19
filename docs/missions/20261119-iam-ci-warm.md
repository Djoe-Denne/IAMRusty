# IAM CI: cold versus pre-existing compiler cache

This qualification is isolated to `ci-iam-warm`. Both arms run independent
server and test producers with empty Cargo targets. They share the application
and SDK revisions, locked Cargo downloads, standard `ubuntu-24.04` runner
class, native companions and prebuilt runtime base. Each arm keeps the same five
IAM coverage partitions and five k6 phases. Coverage waits for its test producer
only; k6 waits for its server, the two shared companions and the runtime base.

- **C (cold):** Both IAM producers start with empty local `sccache` stores.
- **D (warm):** Both IAM producers independently restore the same 311,967,169
  byte snapshot, inside their timed jobs. There is no `target/` import,
  current-run cache upload, shared cache backend or producer-to-producer wait.
  An unavailable or rejected seed fails the warm producer rather than
  silently substituting a cold build.

The only prior artifact allowed is `14527570584` from run `44518086358`,
attempt 1, whose pinned server job `135539506878` successfully exported and
uploaded it. That run is not a qualified comparison: the unrelated warm unit
replay stopped at Ubuntu package installation. Each restore checks run,
repository, producer, upload, expiry and the pinned ZIP SHA-256 before bounded
extraction into the fresh job-local `SCCACHE_DIR`. Executable, runtime and Cargo
dependency artifact admission remains current-run and current-attempt only.

Report compilation and restore times for each producer, their total runner
occupancy, and the arm-level runner occupancy with and without k6 (shared jobs
reported separately). Measure k6 eligibility as completion of its last upstream
producer, actual job start and first scenario, plus coverage and joint
completion. Verify digest-bound LCOV/replay evidence for all 60 targets in each
arm, the exact five k6 phases, SAN restoration and zero remaining
campaign-owned running resources. Distinguish cache hits from net elapsed
savings and record differing hosted runner revisions. The cost of creating
the prior snapshot lies outside the current run; no rollout is authorized.
