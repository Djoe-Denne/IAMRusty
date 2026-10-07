# Telegraph CI investigation — 2026-10-05

Read-only investigation; no Cargo/tests, runtime operations, source edits, commits, project activation, or subagents. Owned runtime IDs: none. Serena: AIForAll/ready. GrepAI index healthy; scoped queries returned empty, unrestricted queries located the correct files. Context Mode handled fetched CI logs; its external-file restriction prevented processing the supplied temp file directly. A short permitted Read confirmed the same failure at saved-log lines 23204–23205.

## Verdict and chronology

**The five reference Telegraph failures are SQS startup failures, not PostgreSQL pool-acquire timeouts.** Run `37228704153`, attempt 1, SHA `178a4cb84cc2b8e6a817971945a1af70989470e4`:

- Saved `37228704153-failed.log`:23204–23205: `test_complex_notification_filtering`, test source :372, `Failed to create test SQS: dispatch failure`.
- Log :23277–23278, :23350–23351, :23423–23424, :23496–23497: source :29/:91/:247/**:180** respectively; `LocalStack failed to become ready after 30 attempts`.
- Log :23575: 0 passed/5 failed, 123.96 seconds. First test failed after ~6.69 seconds; subsequent failures were ~29.31 seconds apart. No startup diagnostic lines in this failing binary. The following routing binary passed (log :23775).
- No `ConnectionAcquire`, `PoolTimedOut`, or `pool timed out` match in fetched failed-run output. The alleged “massive250” pool failure is not established by this reference.

Previous run `37220801796`, attempt 1, SHA `5cbfb315f9b7f19214faeabc9661dfcaf36dd33d`: Telegraph Build succeeded; **both coverage matrix jobs were skipped**. Thus it cannot prove an earlier IT failure. `git merge-base --is-ancestor 5cbfb31 178a4cb` succeeds; Telegraph source and CI workflow have zero delta; both pin SDK `5604074d31da9ba7c81578249ffabd2268cf3c05`. Pool settings are identical before/after SDK Arc change `aa05354`. Conclusion: setup/settings predate the refactor; **failure preexistence and Arc innocence remain unproven**, not grounds to revert Borrow.

## Source evidence (1-based lines)

- `services/Telegraph/tests/common.rs:180–198`, `TelegraphTestFixture::new:118–131`: shared fixture precedes SMTP/server. SDK `rustycog/rustycog-testing/src/common/database.rs:299–343`, `TestFixture::new`: OpenFGA → PostgreSQL/migrations → SQS. The observed failures occur after DB setup, before app build or notification inserts.
- `notification_business_logic_test.rs:25,87,176,243,368`: all five `#[serial]`; each also has its own `#[tokio::test]` runtime. CI `.github/workflows/ci.yml:183–194,283` has no `--test-threads` override. Observed test binaries execute sequentially; singleton containers are process-local.
- SDK `rustycog-db/src/lib.rs:41–54`: pool max32/min5; connect/acquire/idle/lifetime each8 seconds; SQL logging enabled. Test DB creates a pool (`database.rs:65–85`); app creates another (`services/Telegraph/setup/src/app.rs:60`). PostgreSQL `15-alpine` startup has no explicit server max_connections/logging flags (`database.rs:161–183`); readiness uses connect+ping, 30 attempts (`:205–248`). These settings are not saturation evidence.
- SDK `sqs_testcontainer.rs:649–736`: process-global cached container is published before `TestSqs::new` protocol initialization succeeds. `:151–188` readiness checks TCP only, then sleeps2 seconds. `:116–123` queue creation follows. A failed first setup can leave a cached unusable endpoint; crash/OOM/runtime-lifetime versus inadequate readiness needs measurements.
- Telegraph builders `notifications.rs:191–198`, `notification_deliveries.rs:206–213`: awaited single inserts, no retained explicit transaction. SDK `DbConnectionPool::begin_write_transaction`, `lib.rs:128–132`, begins a fresh owned transaction. Preserve borrowed handles, mock non-Clone, Arc for real sharing only.

## Next-phase reproductions — NOT executed

Parent's existing Linux Docker harness, exclusive Cargo lease; normal fixture inputs `RUSTYCOG_TEST_RUNNER_MODE=local`, unique `RUSTYCOG_TEST_RUN_ID` (≤40 alnum/hyphen), existing absolute runner `RUSTYCOG_TEST_LEDGER_DIR`. Verify local mapped-port reachability; otherwise STOP/arbitrate, no silent mode switch. Foreign collisions STOP; opaque startup remains UNKNOWN.

```sh
cargo test --locked -p telegraph-service --test notification_business_logic_test test_complex_notification_filtering -- --exact --nocapture --test-threads=1
cargo test --locked -p telegraph-service --test notification_business_logic_test -- --nocapture --test-threads=1
cargo llvm-cov --package telegraph-service --package telegraph-domain --package telegraph-application --package telegraph-infra --package telegraph-http_server --package telegraph-setup --package telegraph-configuration --locked --no-report --no-fail-fast --tests
cargo llvm-cov report --lcov --output-path lcov-integration-telegraph.info
```

Capture startup logs before first fixture, immutable returned IDs/ledger, mapped endpoints without credentials, exact-ID PostgreSQL/LocalStack timestamped logs and state/OOM/exit information; measure TCP versus successful SQS API readiness. For any genuine pool failure capture size/idle/acquire latency, transaction duration, aggregated pg_stat_activity and SHOW max_connections. Parent reconciles creations on originating runtime and verifies final inventory; no Kind/global cleanup.

If verified: bounded SQS protocol readiness and success-only singleton publication/liveness handling, preserving ownership rules; resource changes only with OOM evidence. Pool/parallelism changes require measured saturation, not guessed green-making. Shared SDK API/publication, lifetime redesign, or runtime-mode/resource decisions require parent/architect/user approval. No assertion weakening, protected Hive changes, or submodule edits.
