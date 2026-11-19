# IAM-only compile-once adoption

Status: Proposed; bounded implementation authorized after the hosted E2E gate.
Parent: [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md).
This specializes the [service-local execution contract](ci-service-local-execution.md)
only for IAM producer compilation, coverage replay and current-CI E2E input.
All other services and all unrelated policy remain unchanged. The
[pilot contract](ci-iam-compile-once-pilot.md) remains the historical experimental envelope.

## Evidence and remaining gate

- The root verified five successful IAM test replays in
  [44351218515](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44351218515),
  source `a359219e0743cbe2cd2b590b573a3906bc20452a`.
- The root verified [44493607389, attempt 1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44493607389):
  source/harness `aba26931ff01de2149587275b5a928ba74194d73`,
  SDK `1db6c9de8708f6451d91a56f482b053026487c8f`; IAM
  `llvm-source-coverage`, OAuth/ext-authz `none`. Exactly `iam-health`,
  `auth-envoy-san-good`, `auth-envoy-san-bad`, `auth-envoy-san-restored`
  and `auth-oauth-browser` passed; restoration and cleanup passed with zero
  owned resources running. Sanitized evidence artifact `14515136213` ZIP SHA-256:
  `38f6aae2b29ae0166e8cd2abaaba61bf7631b4eab8f805ad24f64d96812a9dc0`.
- These different runs qualify the prerequisite separately. They neither establish
  a combined canonical run nor promote experimental LCOV or receipts.
  The user's conditional rollout authorization is now effective; formal ADR
  acceptance and successful full-pipeline qualification are not inferred.

## Producer and scheduling

- Keep `.github/workflows/ci.yml` job `build-iam`, display name
  `Build (IAMRusty)`, its existing selection/trust rules and successful-job checks.
  Keep `build-oauth` / `Build (OAuthConnect)` and
  `build-ext-authz` / `Build (Ext Authz)` unchanged.
- Use one IAM producer target directory for instrumented server/migration and
  the exact test inventory derived from `ops/ci/pipeline.py` and
  `.github/ci/inventory.json`. Select executables from successful Cargo JSON,
  not filenames discovered by scanning `target/`. Compile test executables
  without running tests in the producer.
- Keep the pinned Rust, LLVM, cargo-llvm-cov 0.9.1 and authenticated sccache
  chain. Restrict `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and
  `CARGO_PROFILE_TEST_DEBUG=line-tables-only` to this IAM producer.
  No new feature, test mode, cache policy or cross-service target archive.
  Share compatible units only; do not equate visible Cargo keys or `fresh`
  counts with complete fingerprints or sccache hits.
- Build and freeze the server/migration bundle before test compilation, using
  the same target directory. Transfer bounded executable bundles, never a whole
  target directory. Preserve the existing binary/archive size limits.
- All consumers wait for final producer success, including test compilation and
  bundling. No early-upload bypass, partial-producer admission, rebuild fallback,
  unrelated service dependency or new network service.
  This deliberately adds IAM test-production time to E2E's critical path.

## Coverage contract and fixtures

- Keep the existing `pipeline.py coverage --service ... --lane ... --shard ...
  --output ...` interface, workflow coverage artifact names and canonical
  `coverage_commands` selection/argument policy. Only IAM execution dispatch
  changes to verified binary replay. Record actual producer/replay argv
  separately; never attest that a historical Cargo command ran when it did not.
- Preserve exactly IAM `unit/0` and `tests/0` through `tests/3`, their complete
  disjoint whole-target union, features/defaults, ignored/live-test semantics,
  libtest arguments and failure accumulation. Do not shard individual cases.
  Test and E2E consumers perform no application compilation.
- Preserve package working directories, required package environment, generated
  assets and runtime-library resolution. Each runner retains private profile
  accumulation, its existing fixture preparation, RSA8192 probe material,
  fixture run identity/ledger and owned-resource cleanup. Use an owner-only
  private parent before provisioning or creating sensitive output.
- Emit the existing `aiforall.ci-coverage/v1` receipt and report paths with the
  unchanged identity, scope, inventory/toolchain, features, targets, test/report
  outcomes and LCOV digest semantics. Add producer/executable attestation
  separately; do not rename an experimental receipt into canonical evidence.
  Preserve source-path normalization, exclusions and source mapping policy.
- Missing or invalid input, incomplete/duplicate targets, failed replay or failed
  reporting must remain observable failures. Compatible complete reports may
  merge diagnostically despite failed tests; only full-scope, complete,
  successfully merged evidence with all test/report outcomes successful yields
  canonical `lcov` and Sonar admission. Preserve the existing merger,
  diagnostic manifest and final gate; no subset merge or passing skip.

## Instrumented admission, not delivery admission

- Introduce an explicit current-CI instrumented-IAM artifact/admission contract,
  distinguishable from native v2, pilot and saved-image inputs. Bind repository
  ID/name, allowed event/ref, workflow identity, exact source and SDK, run and
  attempt, successful producer job ID/display name, inventory/toolchain/lock
  digests, partition/member set and executable hashes. Verify API artifact IDs,
  digests, size/expiry and origin before extracting bounded regular members.
  Manifest claims alone cannot establish producer success.
- Preserve `.github/workflows/ci.yml` trusted push/master E2E admission.
  PR test replay may use only its own checked-out CI attempt, without granting
  PRs trusted-push artifact, publication or secret permissions.
  No caller-selectable producer, schema bypass, latest-run or other-attempt fallback.
- Keep native-v2 collectors/validators non-instrumented. Retain release and
  saved-image validation and delivery outputs; where a native delivery build
  is required, it remains separate because its compilation inputs differ.
  Do not conceal wrappers/profile settings or relabel instrumented artifacts.
  To prepare a new native triple after the cutover, manually dispatch `CI`
  on `master`, then pass its successful run and attempt to `k6-images.yml`.
  This explicit dispatch produces the original three native-v2 bundles in
  the same trusted workflow with the original identities. Historical trusted
  push runs remain valid; a dispatch does not run automatic k6 or publish Pages.
- Current-CI E2E may consume only the explicitly admitted IAM instrumented
  server/migration plus the unchanged native companions. Preserve image ABI,
  provenance, the five scenarios, restoration, cleanup and public sanitization.
  Its profiles remain private and never enter coverage aggregation or Sonar.
  Manual saved-image dispatch retains its existing admission unchanged.

## Implementation boundary and validation

Likely implementation surfaces are `.github/workflows/ci.yml`,
`.github/workflows/ci-coverage.yml`, `.github/workflows/k6-e2e.yml`,
`ops/ci/{pipeline,iam_compile_once}.py` and narrowly scoped current-CI adapters
under `ops/tests/k6/ci/`. No application, SDK, migration, fixture behavior,
other-service scheduling, Clippy, Sonar policy, required-check configuration,
selective-PR default, release or cache-backend change.

Before hosted rollout qualification, the root validates separately:

1. Focused `ops/tests/test_ci_pipeline.py` and
   `ops/tests/test_iam_compile_once.py` checks, existing k6 CI refusal tests
   under `ops/tests/k6/tests/`, and actionlint on changed workflows.
   Cover exact target conservation, report/outcome failures, artifact tampering,
   wrong source/SDK/workflow/job/attempt, native/saved-image rejection,
   non-IAM parity and absence of consumer compilation.
2. One complete hosted canonical run at the adopted source/SDK: all five IAM
   partitions, real mapped IAM coverage, unchanged complete declared coverage
   scope and receipts/merge, both Clippy policies, Sonar eligibility, the five
   E2E scenarios, restoration, cleanup and the existing final gate.
   Prior pilot successes cannot substitute for this integrated gate.
3. Producer Cargo phases and reuse evidence, executable/archive bytes, upload
   readiness versus final producer success, queue/setup/download, replay/report
   time, E2E start and whole-run critical path. Compare equivalent scope and
   source with a stated baseline before any speedup claim.

No new architecture approval is required for this bounded implementation.
A failed integrated gate remains a failed qualification, not permission to
weaken receipts, fixtures, trust checks or unrelated service gates.
