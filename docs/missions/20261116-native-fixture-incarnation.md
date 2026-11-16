# Native integration fixture startup: bounded incarnation fix

## Status and scope

The minimal incarnation fix is published in the SDK, and the targeted Hive binary passes. Reliable repeated execution of all five native integration suites is not established: the full-campaign preflight found a separate unsafe Lazaret fixture cleanup path, so neither complete campaign was started.

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
- Images: 99 before and after, no removals. Volumes: 418 before, 419 after, no removals. The new volume's creation event was registered as `ask`; no volume removal was attempted.
- Two consecutive five-suite native campaigns: not started, blocked by the Lazaret safety finding below.

Passing unit or hermetic checks do not establish real dependency startup reliability.

## Separate full-campaign preflight blocker

`services/Lazaret/tests/fixtures/openbao/mod.rs` calls `cleanup_existing_openbao_container()` before the ownership-aware start wrapper. That helper unconditionally executes `docker stop lazaret_test-openbao` followed by `docker rm -f lazaret_test-openbao`. A matching name does not establish ownership. `apparatus_p3_t12_openbao.rs` invokes this fixture without an opt-in gate.

`services/Lazaret/tests/fixtures/redis.rs` also uses the fixed name `lazaret_test-redis`. The existing stopped-name protection means that leaving an earlier process's container until parent cleanup can cause another cross-binary preparation refusal. This is a source-level risk, not a newly executed failure.

Neither fixture was changed or executed in this mission. Starting the full campaign would knowingly enter a cleanup path inconsistent with the user's ownership constraint.

Proposed next bounded change, requiring a separate scope decision:

- Remove the blind OpenBao stop/remove preflight.
- Give the local OpenBao and non-TLS Redis fixtures fresh names, while retaining the existing ownership-aware creation and exact-ID teardown.
- Leave ports, assertions, thread settings, and the SDK ownership backend unchanged.
- Validate that a foreign old-name canary is never stopped/removed and that successive fixture processes cannot reuse a reserved name; then resume the two full native campaigns.

## Resource boundary

Before native validation, six pre-existing containers were inventoried: three running local-full Kind nodes, one exited MailHog container, and two Created PostgreSQL containers with unknown ownership. None belongs to this task. Do not stop, remove, adopt, or reclassify them.

The existing harness records fresh container identities and performs its exact-ID cleanup. Supplementary read-only observation records image/volume creation events as `ask`, without granting deletion permission. No global cleanup, port-allocation wrapper, concurrency reduction, runtime-wide shutdown, or foreign-resource removal is authorized.

## Evidence and continuation

- Prior continuity: `.cursor/handoffs/20261115-fixture-fresh-start.md`.
- Current continuity: `.cursor/handoffs/20261116-native-fixture-incarnation.md`.
- Current evidence: `.tmp/fixture-incarnation-20261116/`.
- Historical evidence: `.tmp/retire-legacy-connect-20261115/integration/` and `.tmp/postgres-daemon-port-20261115/`.
- Resource registry: `.tmp/docker-resources-20261116.log`, plus all relevant historical daily registries.

No new native preparation failure occurred in the targeted run. The historical port failure remains unresolved; a successful Hive binary does not disprove it. Resume full-campaign validation only after the Lazaret safety blocker has an approved disposition. Do not automatically rerun the campaign or add a speculative port fix.
