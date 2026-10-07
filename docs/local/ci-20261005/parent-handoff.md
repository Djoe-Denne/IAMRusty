# Non-IAM CI work — parent handoff

Original baseline: `9ddb33bbea6500032f26ccfde648bed51b776bdf`; reference CI `37228704153`.
User-owned commit observed during work: `87a6a10fa20445983a24535c4e6775def745a5ac` (`test(iam): load RSA-8192 probe material from env instead of hot keygen`). Do not reset, edit or stage the user's IAM/key work. IAM tests remain explicitly suspended until user authorization resumes them.

## Current verified state

- Reference CI: 18 successful jobs, 5 failed Coverage jobs (IAM unit+integration, Telegraph integration, Manifesto integration, Hive integration).
- Redundant docs CI `37330477859`: cancellation completed.
- IAM: no source edits by this task; four historical assertion failures reproduced before suspension. Historical on-the-fly RSA8192 probe passed in 224.94s on two CPUs, before the user's new environment-variable solution.
- Manifesto: 12 test path fixes applied. Raw local logs independently checked: 38 guard passes, 5 newly exposed lexical failures; filesystem NotFound failures gone. Architecte is reconciling these against the existing guard contracts, not weakening scans.
- Hive: exact verifier-contract assertions aligned to the configured RS256/HS256 pair. Raw real-harness test log checked: 1 pass, 7.10s. Protected test/fixture files have no diff.
- Exact Manifesto and Hive matrix builds: raw logs checked successful (3m34s, 3m03s). Workspace fmt was reported successful; final parent recheck still due after remaining source work.
- Telegraph: prior investigation identifies SQS/LocalStack dispatch/readiness setup failures, not a proven PostgreSQL pool-acquire issue. Previous CI skipped coverage, so preexisting failure chronology is not established. Reproduction is delegated under the sole Cargo lease.
- Full matrix and exact SonarCloud commands remain deferred; no task commit/push, no claim of fully green CI.

## Original-request reconciliation after user key work

GitHub metadata confirms repository secret `IAMRUSTY_TEST_RSA8192_PEM_B64`, updated `2026-10-05T15:59:07Z`; no value read or decoded. User-owned `87a6a10` installs static-material loading and CI injection, not corrections to the four historically failing IAM files.
After explicit user go, metadata-only local check confirms `IAMRUSTY_TEST_RSA8192_PEM_B64` is present in the inherited Process environment (User/Machine scopes absent). Existing material can be passed through by environment-variable NAME to the Docker test process, without printing the value, decoding it in model context, putting it in CLI arguments or regenerating RSA8192.
The four remaining IAM causes require contract-preserving fixes/reruns: effective signing binding equality; rotate 400 vs expected 200; safe OAuth log capture (the expected static log text still exists, so S7 removal is not established); PEM CRLF acceptance/mismatched-pair protection. Other IAM binary bootstrap failures still need individual reproduction before applying the valid RSA2048/opaque-kid/live-PEM pattern.
Loader coverage also needs verification of actual execution and 8192-bit size: metadata alone does not prove size; the current unset-env path can return without exercising the probe.
Current remote run `37340002817` on `87a6a10`: Telegraph integration SUCCESS; IAM unit FAILURE; Hive and Manifesto integration FAILURE (our fixes are still uncommitted); IAM integration still active when observed. Logs cannot be retrieved through gh until that run completes; no claim about exact current failed-test set.
Final remote observation supersedes the preceding live snapshot: `37340002817` is COMPLETED/FAILURE, with four red Coverage jobs (IAM unit, IAM integration, Hive integration, Manifesto integration). The downloaded current failed log `37340002817-failed.log` independently confirms all four core IAM panic locations in BOTH unit and integration; integration also still logs invalid-signing-material bootstrap errors. RSA8192 probe is reported OK, but its current early-return path means that label alone does not prove 8192-bit material was exercised.
Telegraph exact local Coverage completed exit0, 50 PASS/0 FAIL across 18 target summaries; exact build and LCOV reporting PASS. No Telegraph/SDK/pool edit justified. Parent independently inspected 30 returned-ID SDK fixtures plus one returned-ID SMTP fixture: 31/31 exited. Four Hive fixtures also independently verified exited.
Final 43 guards + 6 attach tests completed: raw logs independently confirm 49 PASS / 0 FAIL; exact Manifesto build, fmt and scoped diff-check PASS. Remaining before publication: required full exact build-matrix gates, both Sonar clippy commands, affected final unit/IT/Coverage after IAM corrections, commit/push and successful CI watch. Original secondary backlog stays after green: strict org-claim 403 plus SDK alignment, T1 one-shot end-to-end proof, T8 schema snapshots.
Loader review confirms a MEDIUM coverage gap: `.cursor/review-briefings/20261005T1719Z-tests-rsa8192-env-probe-87a6a10.md`. Empty/missing env silently bypasses both tests; loaded key size is not asserted. Static generation need not change, but 8192-bit assertion and nonempty CI-secret guard are needed to preserve the original tested boundary. Secret size/actual probe execution were not read or inferred from metadata.
All owned unused runtime released: compiler `493026f9bfc97cf2a64e768f4d7ab198875c4b72d62f8f7bb84fe0fe320ad805` stopped by exact ID and independently verified exited. Final running inventory contains only the two pre-existing foreign Kind nodes; no Docker Desktop/WSL shutdown because a protected workload remains. Measured vmmemWSL 12,098,617,344 bytes, host free physical memory 2,263,440 KiB; named caches preserved.

## Reviews / residual ruling

- Correctness PASS: `.cursor/review-briefings/20261005T1645Z-correctness-manifesto-hive-non-iam-87a6a10.md`.
- Test coverage PASS: `.cursor/review-briefings/20261005T1644Z-tests-manifesto-hive-paths-verifier-87a6a10.md`.
- Security (Hive slice) PASS: `.cursor/review-briefings/20261005T1646Z-security-hive-signer-verifier-87a6a10.md`.
- Architecte permits four exact textual substitutions to remove the five remaining lexical hits, no ADR change and no scan/allowlist weakening: `manifesto-guard-contract.md`. Mechanical edits delegated; Rust execution of the final guards/module remains due after Telegraph releases Cargo.
- The four textual edits are complete; incremental correctness PASS: `.cursor/review-briefings/20261005T1652Z-correctness-lexical-residue-87a6a10.md`. The initial 13-file review stays distinct. Rust execution of 43 guards + 6 attach tests is still pending the sole Cargo slot.
- Incremental test-coverage PASS: `.cursor/review-briefings/20261005T1653Z-tests-lexical-residue-87a6a10.md`; static guard sensitivity remains intact. This does not substitute for the pending Rust execution.

## Runtime and ownership

Compiler: `493026f9bfc97cf2a64e768f4d7ab198875c4b72d62f8f7bb84fe0fe320ad805`, created absent before the session and proven by worker creation-ID ledger, rust:1.94-slim-bookworm, named Linux caches. Retain only for active delegated validation, stop gracefully before final handback when no active need remains.

Four returned-ID Hive fixtures independently inspected and all `exited` after graceful stops:
- `41a94932d96daa1edbc4d4ad1b67550a49ab4727b156dfef4398637b497a36ee`
- `6f560a46a804ee02d7532ff3471f24a31049eaf4f521849f3fac15c17348e61e`
- `6bd37026460f40bc246699bb1215825431ebd1904ef35d5997db3824e6ec3d38`
- `c9aab227738202c297547df6070cf030676fb2b6058d10fb9f152570d0857b44`

Baseline running Kind IDs are foreign, one protected Apparatus: never stop them. No global Docker Desktop/WSL shutdown while these workloads remain.
Authoritative operational lease/IDs: `C:/Users/djden/AppData/Local/Temp/opencode/ci-20261005/session-ledger.json`.
Local artifacts here are not part of the proposed source commit.
