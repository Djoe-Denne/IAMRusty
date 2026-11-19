# IAM instrumented binary: hosted functional E2E qualification

## Result

[GitHub Actions run 44493607389, attempt 1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44493607389) succeeded on `ci-iam-compile-once` at `aba26931ff01de2149587275b5a928ba74194d73`. Both the source and harness were that commit. The SDK pin was `1db6c9de8708f6451d91a56f482b053026487c8f`.

The IAM producer instrumented both the server and migration executable with LLVM source coverage. OAuthConnect and Ext Authz stayed non-instrumented. The E2E job acquired the immutable bundles from this same attempt, verified and packaged them, and ran the existing campaign without Cargo.

| Phase | Evidence |
| --- | --- |
| `iam-health` | Completed |
| `auth-envoy-san-good` | Completed |
| `auth-envoy-san-bad` | Completed |
| `auth-envoy-san-restored` | Completed |
| `auth-oauth-browser` | Completed |
| Campaign, restoration and cleanup | PASS; zero owned resources running |

The API identifies the sanitized evidence as artifact `14515136213`, SHA-256 `38f6aae2b29ae0166e8cd2abaaba61bf7631b4eab8f805ad24f64d96812a9dc0`. The ZIP digest was checked before its three members (`report.json`, `provenance.json`, `cleanup.json`) were parsed. The public report identifies internal campaign `4c90d969562f45e092634670c0ad6196`.

## Timing and limits

The IAM producer took 112 seconds; the E2E job took 731 seconds including artifact acquisition, image creation, tooling, runtime and cleanup. The successful qualification establishes functional compatibility, not a CI speedup or release equivalence. The test-executable partitions previously passed as separate replay jobs in [run 44351218515](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44351218515); this E2E-only run did not recompile or rerun those partitions.

The preceding [run 44492337304](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44492337304) reached image packaging but never reached the E2E campaign because its launcher lacked the owner-only parent required for tool provisioning. The passing run restored that wiring without relaxing the private-directory guard or changing the scenarios.
