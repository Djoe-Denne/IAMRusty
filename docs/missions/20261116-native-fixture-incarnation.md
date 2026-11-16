# Native integration fixture startup: bounded incarnation fix

## Status and scope

The minimal incarnation fix and Lazaret safety correction are published. Two consecutive complete five-suite campaigns pass their configured tests on unchanged sources: 820 passed and one pre-existing ignored test per campaign, with no preparation failures. Container removal is not complete: the existing harness retained nine stopped MailHog containers per campaign with nonzero exits for diagnosis. The historical PostgreSQL port conflict remains unexplained and was not reproduced.

The user approved a minimal naming fix and bounded validation. The withdrawn port-allocation rewrite is not restored, and its passing tests are not evidence for this change.

## Preserved baseline

- AIForAll baseline: `8379f5cd5869ab7190c9c5c4585f255f9a698386`, with pre-existing uncommitted connector-retirement and IAM fixture-path changes preserved.
- Initially consumed SDK: `54af36c22047b995a4f362f3907abeadbc858ec2`.
- SDK development baseline: `0657936f7a77f8abdc0210bfd6aa1ba7de897428`, whose complete source tree is identical to the initially consumed SDK.
- Execution remains Windows-native, with one Cargo process, 24 configured build jobs, target directory `E:\cargo-target\AIForAll`, and unchanged integration-test thread settings.

## Functional findings

### Name collision

The historical IAM refusal inventory contains an exited PostgreSQL container with exactly the name requested by a later process incarnation. The two processes share the same campaign ID, PID, and attempt counter. Their ledger filenames differ by incarnation UUID, but the retained container-name formula omits that UUID. The matching historical container has no published ports in that inventory.

The process-exit guard stops proven-owned containers without removing them. Parent cleanup removes eligible containers at campaign end. Consequently, a stopped container can still reserve a name when Windows reuses a PID between integration-test binaries. Rejecting that existing name is correct; generating the same name for the new incarnation is the defect.

This historical observation comes from a withdrawn experimental campaign. The naming defect was separately reproduced in a deterministic test against the retained formula, without Docker. This does not validate the removed implementation.

### Port-publication failure

The original retained-SDK Hive campaign failed during fixture startup, before application assertions, when Docker refused publication of PostgreSQL port 60884. Its preceding Docker inventory has no matching name or published port. No competing port owner has been established.

The retained allocator briefly probes a Windows loopback port, closes the socket, and later asks Docker to publish the number. This leaves a possible conflict window and does not prove Docker-side availability. It is not sufficient evidence to identify the historical cause. Port selection, cache invalidation, endpoint propagation, readiness, and ownership logic remain unchanged by this fix.

## Minimal implementation

SDK commit: `788da34a7d2570ebddbc5c2de1c56b99c177cec2`, published and verified on `origin/main`.

- Keep the existing process-incarnation UUID in the internal fixture context.
- Use that same UUID in automatically generated fixture names and ledger filenames.
- Preserve explicit service-local names, collision refusals, exact-ID ownership, and cleanup rules.
- Strengthen the existing ledger-incarnation regression to exercise generated container names and preserve old stopped-container name reservations.
- Add a maximum-input name-length/character regression.

AIForAll adds only the exact Hive `organization_api_tests` selection to the existing native launcher. Its tests cover malformed/cross-suite refusals and the actual Cargo argument branch. No application assertion is weakened.

## Validation

- Deterministic RED: the strengthened regression fails with the retained naming formula because two incarnations produce the same name.
- SDK fixture-runtime GREEN: 24 tests passed, zero failed or ignored.
- Rustfmt on affected SDK files and `git diff --check`: passed.
- Native-launcher selection and thread-policy checks: passed.
- Hermetic launcher stop and fail-closed checks: passed. The initial invocation failed before fake Cargo because the Context Mode subprocess environment omitted the host `PATHEXT`; rerunning with that observed host environment passed, without changing repository or global configuration.
- Targeted native Hive `organization_api_tests`: 23 passed, zero failed, ignored, or filtered; unchanged two libtest threads. Campaign `it-fdc11a6a3ef1429a8ddeea8f25900f1d`.
- Targeted cleanup: two proven-owned containers created, both absent afterward; zero owned running, zero unknown creations, zero collision events. The six pre-existing containers retain their IDs, names, states, images, published ports, and mounts in the before/after snapshots.
- Images: 99 before and after, none missing. Volumes: 418 before, 419 after, none of the pre-existing volumes missing. The new volume's creation event was registered as `ask`; no additional volume-cleanup command was issued.
- Two consecutive complete five-suite native campaigns: configured tests passed on identical sources. See the separate cleanup qualification below; neither campaign establishes full fixture removal.

Passing unit or hermetic checks do not establish real dependency startup reliability.

## Separate full-campaign preflight blocker

At the initial preflight, `services/Lazaret/tests/fixtures/openbao/mod.rs` called `cleanup_existing_openbao_container()` before the ownership-aware start wrapper. That helper unconditionally executed `docker stop lazaret_test-openbao` followed by `docker rm -f lazaret_test-openbao`. A matching name does not establish ownership. `apparatus_p3_t12_openbao.rs` invokes this fixture without an opt-in gate.

The plaintext fixture in `services/Lazaret/tests/fixtures/redis.rs` also used the fixed name `lazaret_test-redis`. The existing stopped-name protection means that leaving an earlier process's container until parent cleanup can cause another cross-binary preparation refusal. This was a source-level risk, not a newly executed failure.

The first phase stopped before running these fixtures. Starting the full campaign at that point would have knowingly entered a cleanup path inconsistent with the user's ownership constraint.

The user then approved this separate bounded correction:

- Remove the blind OpenBao stop/remove preflight.
- Give the local OpenBao and non-TLS Redis fixtures fresh names, while retaining the existing ownership-aware creation and exact-ID teardown.
- Leave ports, assertions, thread settings, and the SDK ownership backend unchanged.
- Validate that a foreign old-name canary is never stopped/removed and that successive fixture processes cannot reuse a reserved name; then resume the two full native campaigns.

### Lazaret follow-up implementation and validation

The blind OpenBao helper is removed, and the local OpenBao and plaintext Redis fixtures now generate UUID-suffixed names. Their existing port selection, singleton behavior, ownership-aware wrapper, readiness probes, and exact-ID teardown are unchanged. The TLS Redis fixture and original protocol assertions are untouched.

`services/Lazaret/tests/fixture_name_isolation.rs` creates two controlled canaries through the existing supervised harness, reserving the legacy OpenBao and Redis names. These resources belong to the regression test, not to the fixture under test. The new fixtures must remain usable without changing either canary's real ID, running state, or sentinel Redis data. Inspect-by-name is used only as an assertion witness, never as cleanup authority. No existing foreign container is adopted or removed.

- Rustfmt on the affected files: passed.
- `cargo test --locked -p lazaret-service --tests --no-run`: passed, compiling 16 executables.
- Full native Lazaret suite: 81 passed, zero failed, ignored, or filtered across 16 result summaries; two unchanged libtest threads. Campaign `it-7d385f2109d444b382f687a078a57dad`.
- The isolated canary regression passed: old-name canaries retained their real identities, running states, and sentinel data while new OpenBao and Redis fixtures remained usable.
- Actual process-boundary evidence: two OpenBao containers across two processes and three plaintext Redis containers across three processes, all with distinct names.
- Fifteen proven-owned containers created, all absent afterward; zero unknown creations or collision events. All six pre-existing containers retained their IDs, names, states, images, ports, and mounts.
- Images remained 99 -> 99; volumes 419 -> 433, with no pre-existing resource missing. New volume events remain recorded as `ask`, without inferred cleanup authority. Transient volume events are accounted for separately below.
- Lazaret correction published as `61e8bcdd8ca82414900cfaac5630136996a855ee`, verified on `origin/master`. The SDK remains `788da34a7d2570ebddbc5c2de1c56b99c177cec2`.

## First complete five-suite campaign

Run `it-9315994794204643bbc6a1160eba7853`, evidence `.tmp/fixture-incarnation-20261116/full-1/`, completed on the unchanged source with exit 0. All five suite markers were verified in order:

- Hive: 78 passed.
- IAM: 303 passed, one ignored.
- Manifesto: 283 passed.
- Lazaret: 81 passed, including the canary regression.
- Telegraph: 75 passed.

Total: 820 passed, zero failed or filtered. IAM's ignored `test_signup_sqs_integration` already carries the `requires SQS LocalStack testcontainer` annotation in baseline `8379f5cd`; no new ignore, feature change, or assertion reduction was introduced.

The campaign created 207 proven-owned containers. Of these, 198 were absent afterward and nine MailHog containers remained stopped with exit code 2. Zero owned containers remained active or unsettled. There were no new unknown creations, opaque starts, or collision refusals. All six original containers retained their IDs, names, states, images, published ports, and mounts.

The nine stopped MailHog containers are retained as `ask` under the existing nonzero-exit diagnostic policy. Selected live inspection confirms exit 2, no OOM flag, and an empty Engine state-error field; the selected log filter found no fatal/panic marker. The cause of exit 2 is not established. These are not evidence of a fixture-preparation failure and must not be silently removed or reclassified. The strict all-containers-removed verification remains false; configured-test success and zero-active/unknown resource safety are separate results.

All 155 automatic names checked against their ledger-incarnation UUIDs match. Images remained 99 -> 99; volumes 433 -> 545, with all 112 net additional volume names registered as `ask` and no pre-existing resource missing. The second planned repetition proceeded without changing sources or cleaning the retained diagnostics; its baseline preserved all 15 existing containers.

The first full campaign exercised real PID reuse, not only a synthetic regression: PID 32136 appears in two ledger incarnations, with OpenFGA attempt 0 and PostgreSQL attempt 1 in both. The later inventories still contain the earlier containers, positively exited. Their old-formula names would be identical, whereas the actual incarnation-suffixed names differ and the new creations succeed without removing the predecessors. This directly exercises the corrected identity boundary.

All nine retained MailHog IDs are the final SMTP creation in their respective nine ledger contexts. The fixture cleans its preceding owned singleton before each new creation; the process-exit guard only stops the final instance, and parent cleanup retains nonzero exits. That accounts for which instances remain, without establishing the reason for exit code 2.

## Second complete five-suite campaign

The second campaign completed with exit 0 on the same source, including the same preserved uncommitted user changes. Its timestamps, campaign ID, and full resource identities are archived in `.tmp/fixture-incarnation-20261116/full-2/{status,verification}.json`.

Results are identical to the first campaign: Hive 78, IAM 303, Manifesto 283, Lazaret 81, and Telegraph 75 passed; total 820 passed, zero failed or filtered, and the same single pre-existing ignored IAM SQS test. The canary regression passed again.

The second run began with 15 containers: the six original resources plus the nine retained first-run diagnostics. All 15 retained their IDs, names, states, images, published ports, and mounts. It created 207 new proven-owned containers, with 198 absent afterward and nine new MailHog containers retained exited(2), classified `ask`. Zero new owned containers remained active or unsettled; no unknown creation, opaque start, collision refusal, or ownership conflict was recorded.

All 155 automatic names checked against the second run's ledger-incarnation UUIDs match. Images remained 99 -> 99. Volumes increased 545 -> 657; all 112 net additional names were registered as `ask`, with no pre-existing resource missing.

### Result and limits

- Repeated configured-suite startup, application-test completion, and zero-active/unknown resource safety are demonstrated by these two runs. They are not a guarantee against every future port conflict.
- The tests include the preserved, uncommitted connector-retirement and IAM fixture-path changes. `61e8bcdd` identifies the committed mission code, not a pristine checkout containing that entire tested working tree. The preservation manifest and source hashes identify this local evidence boundary.
- Across both complete runs, 414 new containers were created, 396 were absent after their respective runs, and 18 MailHog containers remain stopped for diagnosis. Earlier targeted validation created and removed another 17 containers. No task-owned container is expected to remain active.
- The strict all-containers-removed verifier verdict is false for both complete runs. This is separate from passing application tests and ownership-safety checks; no cleanup assertion or nonzero-exit retention rule was weakened.
- All recorded new volume creations remain `ask`; no additional volume/image cleanup command was issued. Normal fixture execution also emitted transient volume-destruction events, described below. Retention is explicit, not a claim of full fixture disposal.
- Do not add another implementation or rerun merely to obtain a cleaner administrative result. The historical port conflict and MailHog exit-2 cause require separate bounded evidence if further investigation is requested.

## Final resource and preservation check

The final live inventory confirms 431 distinct container IDs created across the targeted and complete validations: 413 absent and 18 MailHog containers stopped with exit code 2, all retained as `ask`. No owned container is running; no new unknown container remains. The original six containers retain their identities, names, states, images, ports, and mounts.

Volume observation recorded 248 creation events and nine destruction events across the four validation windows. All nine destroyed names were created during those windows; none belonged to the initial 418 volumes. Of the observed new names, 239 remain present and nine are absent, with no unexplained absence. The 657-volume final snapshot therefore consists of the original 418 plus 239 net additional volumes. Snapshot “removed” counts concern pre-existing resources, not all transient operations. No extra cleanup or ownership inference was used to obtain these counts.

The 101 pre-existing user status entries and available content hashes remain identical. No task branch, worktree, or agent team was created. Docker Desktop/WSL and the three pre-existing local-full nodes remain untouched; no runtime-wide shutdown or RAM-release claim is made. The final observed `vmmemWSL` working set is approximately 10.2 GiB, belonging to the retained shared runtime rather than active task containers.

Detailed final identities and retention statuses are in `.tmp/fixture-incarnation-20261116/final-state.json`; volume accounting is in `volume-lifecycle-summary.json`. Full removal of fixtures is not certified, and the remaining `ask` resources require a separate scoped disposition rather than an automatic clean.

## Resource boundary

Before native validation, six pre-existing containers were inventoried: three running local-full Kind nodes, one exited MailHog container, and two Created PostgreSQL containers with unknown ownership. None belongs to this task. Do not stop, remove, adopt, or reclassify them.

The existing harness records fresh container identities and performs its exact-ID cleanup. Supplementary read-only observation records image/volume creation events as `ask`, without granting deletion permission. No global cleanup, port-allocation wrapper, concurrency reduction, runtime-wide shutdown, or foreign-resource removal is authorized.

## Evidence and continuation

- Prior continuity: `.cursor/handoffs/20261115-fixture-fresh-start.md`.
- Current continuity: `.cursor/handoffs/20261116-native-fixture-incarnation.md`.
- Current evidence: `.tmp/fixture-incarnation-20261116/`.
- Historical evidence: `.tmp/retire-legacy-connect-20261115/integration/` and `.tmp/postgres-daemon-port-20261115/`.
- Resource registry: `.tmp/docker-resources-20261116.log`, plus all relevant historical daily registries.

No new native preparation failure occurred in the targeted validations or either complete campaign. The historical port failure remains unresolved; successful runs do not disprove it. The planned repetitions are complete, with retained diagnostic containers left untouched. Do not automatically rerun a failed campaign, weaken the nonzero-exit retention policy, or add a speculative port fix.
