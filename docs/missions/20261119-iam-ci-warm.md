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

## Hosted result — run 44522586890, attempt 1

[The isolated run](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44522586890)
at `615ed05befe3a55f86f7eb5bf3adbf009474a5c3` finished at **11:55**
with nine successful jobs, five failures and three skips. This was not a
qualified comparison: Ubuntu mirror errors occurred before the measured
operations in multiple jobs, so neither arm completed coverage and k6.

- The [common runtime base job](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44522586890/job/135554737068)
  failed while downloading Ubuntu package indexes and published no image.
  Neither k6 job reached eligibility or started a campaign. There is no
  current-run k6, SAN restoration, or cleanup receipt; those checks did not run.
- The cold test producer and the warm server producer failed before compilation
  because downloading `libcurl4-openssl-dev` returned HTTP 503. In particular,
  the warm server never reached its restore step. Warm coverage partitions
  `tests-0` and `tests-2` failed before replay on connection errors fetching the
  same package. Cold coverage never started.
- The [warm test producer](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44522586890/job/135554964663)
  independently started at **0:28**, restored the exact pinned seed by digest
  (**412 entries, 298 MiB ZIP**) and used **287 cache hits / 286 misses**.
  Download, verification and extraction together took **9 s**. Cargo took
  **222.1 s** and the entire producer occupied its runner for **6:12**,
  including restore and bundle upload. Its five test bundles were published.
- The cold server built successfully with **0 hits / 412 misses** in
  **128.9 s** Cargo time and **2:55** total runner occupancy. Its counterpart
  did not compile, and the cold test producer did not compile, so neither
  measurement has a same-run cold/warm counterpart. Observed combined
  producer occupancy, including failed setup, was 511 s cold and 939 s warm.
  This is not a speed comparison: the failed package install alone took
  300 s in the cold test job and 530 s in the warm server job.
- All six compiler jobs verified the same locked Cargo-download seed, artifact
  `14529375487`. The two successful IAM producer jobs used the same runner
  image revision (`20261111.370.1`), but measured different build workloads.
  Verified all six available small ZIPs against their API SHA-256 digests and
  checked matching LCOV and replay receipts from the exact attempt. Three
  warm partitions ran successfully: `unit-0` (7 targets), `tests-1` (14)
  and `tests-3` (12), for **33 distinct targets**. No full coverage or joint
  completion time exists for either arm.

**Disposition:** the independent warm restore demonstrably worked for the
test producer; its net effect on speed is inconclusive. No retry, merge or
promotion. If revisiting this experiment, stabilize the common Ubuntu package
inputs before a fresh paired run, keep both arms in one attempt, and verify
seed availability before launching. Do not join timings from different runs
and present them as an equivalent pair.

## Follow-up: isolate Ubuntu package delivery from the paired jobs

The new `prepare-apt` job resolves `libcurl4-openssl-dev`, its version-matched
`libcurl4t64` and any additional packages that APT needs. It first uses the
official Canonical HTTPS archive and security repositories; a failed refresh
or download gets one attempt on the MIT Ubuntu mirror, with fresh indexes and
empty download staging. Both use the Ubuntu archive keyring, strict index
refresh, one retry per item, 15 s inactivity timeouts and bounded commands.
No actual installation is attempted during mirror selection.

All four IAM producers and ten coverage jobs use the same immutable
current-attempt ZIP, installed without network access after digest checks.
Each logs the digest and installed package versions. Coverage and k6 remain
independent of each other. The shared k6 base still uses the pinned Ubuntu
image and the same three runtime packages, but stages them together with the
same mirror choice before installing using APT's local archives. The shared
preparation durations and both arms' artifact install time must be included
when measuring timing. This does not assert that unrelated runner-image
libraries are byte-identical and does not change normal CI behavior.
