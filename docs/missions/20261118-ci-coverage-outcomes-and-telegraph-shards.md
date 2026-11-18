# Coverage outcome isolation and Telegraph partitions

## Request and decision

The user requested a behavioral correction: failed Telegraph tests must not themselves make coverage merging fail. The earlier `b3bf7025` diagnostic-only change clarified the cause but did not implement that separation. The user also requested trying smaller partitions for Telegraph's third coverage block.

The forward amendment to [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md) and the [execution contract](../contracts/ci-service-local-execution.md) separate report integrity from test verdict. ADR-0507 remains Proposed. Commit/push is authorized; no application/SDK, billing/quota, live-test opt-in or remote required-check change is included.

## Coverage behavior

- Validate the complete source/run/attempt-bound report set, policy/toolchain, features, assignments, paths and digests independently of test success.
- If all reports are valid and LCOV merges successfully, publish `coverage-diagnostic` with `diagnostic-lcov.info` and `coverage-summary.json`, including all partition outcomes and qualification reasons.
- A nonzero test outcome does not stop this diagnostic merge. It does make successful execution unproven; Cargo exit101 can be a compiler or assertion failure.
- Keep test jobs failed. Do not rewrite receipts, omit partitions or add blanket `continue-on-error`.
- Only all-success, complete full-scope evidence additionally produces canonical `lcov`. Sonar requires its explicit qualification output and verifies the manifest/report without invoking Cargo.
- A diagnostic-only merge deliberately skips Sonar. Final status explains that distinction while retaining actual test or unrelated failures.
- Missing/duplicate/foreign evidence, report-generation failure, empty/corrupt reports or actual LCOV failure still block merging.

## Telegraph layout

The original unit partition and original `tests/0` are unchanged. The former `tests/1` contained eight binaries and 66 observed cases; it is now split as:

- `tests/1`: `notification_created_routing_test`, 17 baseline cases.
- `tests/2`: `coverage90_process_event`, `template_and_delivery`, `common`, `https_mesh_optional_mtls`, `notification_http_endpoints_test`, `outbox_tests`, 41 baseline cases together.
- `tests/3`: `user_email_verified_event_test`, eight baseline cases.

One shared assignment helper drives matrix discovery, Cargo target selection and receipts. This is whole-binary partitioning, not test-case sharding. Existing features, libtest concurrency and fixtures are unchanged. Future targets remain assigned rather than silently disappearing.

The real native metadata audit confirms 59 packages, the same 193 unique baseline coverage targets and 21 partitions, compared with the former 19. The preserved first integration group has ten binaries; the new integration binary counts are 10/1/6/1.

The original eight binaries spent 80.42 seconds in tests on the observed run, excluding compilation/setup. Smaller groups may aid isolation but do not establish a global speedup or fix the underlying intermittency.

## Validation status

- Final focused suite: 32 tests passed, covering diagnostic vs canonical outcomes, typed/negative admission, stable partition conservation, canonical verification without Cargo, final status and qualification-output emission. Tests isolate the runner output-file environment.
- Existing CI/E2E static seams: 133 tests total, 132 passed and one opt-in skip.
- Actionlint passed for the CI and reusable coverage workflows.
- Real metadata/assignment conservation audit passed.
- Independent read-only correctness and security reviews passed without findings. Consumer verification trusts the same-run aggregation producer; it does not independently reconstruct Cargo metadata.
- Normal source/diff checks passed. Hosted LCOV execution, artifact publication and new-layout test outcomes are not yet qualified.

Named logs and disposable evidence are retained under ignored `.tmp/ci-coverage-isolation-20261118/` and `.tmp/ci-cache-20261117/`. No local application compilation or Docker/Kind/WSL runtime was started. Unit merger stubs verify control flow, not the real LCOV parser.
