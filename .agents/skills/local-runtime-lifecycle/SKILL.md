---
name: local-runtime-lifecycle
description: Governs on-demand Docker Desktop, Kind and WSL startup and session-owned cleanup for E2E, requested manual/local tests and integration tests (IT/testcontainers), task completion/failure and idle handback. Use before starting or ending local test runtime, recovering owned resources, or reclaiming WSL RAM; guidance only, not an inactivity monitor.
---

# Local runtime lifecycle

## Scope and references

Apply the short canonical rule [local-runtime-lifecycle.mdc](../../../.cursor/rules/local-runtime-lifecycle.mdc) and [AGENTS.md](../../../AGENTS.md); [infra-safety.mdc](../../../.cursor/rules/infra-safety.mdc) remains mandatory. Never operate on `kind-apparatus-p4-it`, `rancher-desktop`, or production. A prior one-off authorization for a protected cluster does not grant future permission.

- Local deployment conventions: [ops/deploy/README.md](../../../ops/deploy/README.md), [Kind config](../../../ops/deploy/kind/cluster.yaml). Do not execute their delete/recreate suggestions automatically.
- IT contract: [ADR 0200](../../../docs/adr/0200-it-infra-reelle-rustycog-testing.md), [testcontainer fixtures](../creating-testcontainer-fixtures/SKILL.md), [outbound HTTP fixtures](../creating-wiremock-fixtures/SKILL.md).
- Docker operations: [Compose](../docker-compose-patterns/SKILL.md), [builds](../docker-build-strategies/SKILL.md), [destructive guardrails](../docker-destructive-guardrails/SKILL.md). Fixture runbook cleanup/removal examples do not authorize deleting unknown/old resources or `rm -f`.

## 1. Gate and assign ownership before startup

1. Identify the requested E2E/manual local test or actual validation need. Documentation/static work does not require waking Docker/WSL/Kind. Respect a stopped runtime until such a need exists.
2. The orchestrator/active parent owns a shared resource lease across workers and ongoing background validation. Record the request, session/task owner, active consumers, expiry/release condition and any explicit user keepalive. A merely open/idle session or speculative next test is not an active need.
3. Record a BEFORE inventory using observations that do not wake stopped daemons: Docker Desktop/WSL states, known workloads/leases, and running/stopped container IDs and cluster/context mapping when Docker is already available. If startup is required, record stopped state first, then inventory containers immediately after daemon availability and before starting test resources. Unknown ownership is not permission.
4. Track each CREATED or explicitly leased/restarted container by exact ID, prior state, action, cluster (if any) and request. Existing resources require an explicit scoped lease before restart; never infer ownership from image/name or claim old resources as session-owned. Persist a compact non-secret ledger in task/session recovery notes before long operations, updating it as IDs become known. Do not store credentials/full inspect dumps.
5. Route Docker/Desktop/WSL to `container-runtime-debugger`, Kind to `k8s-operator`, network issues to `envoy-network-debugger`, verification to `infra-verifier` per AGENTS. Leaf workers cannot delegate: report needed operations to the parent. Do not bypass tool permissions.

## 2. Start only the selected runtime

- **E2E or requested manual local test:** use cluster `aiforall-local`, explicit context `kind-aiforall-local` only. Reuse its stopped, identified node containers under an explicit lease (graceful start, not recreate); if absent, use the repository's create-if-absent convention/config. If reuse fails or CNI/config conflicts, stop and escalate; no automatic delete/recreate/reset. Start only dependencies required by the selected repository scenario.
- **IT:** run the existing real testcontainers harness; fixtures autostart their protocol dependencies. Never start Kind for IT or migrate fixtures to Compose. A legacy test requiring a protected Kind fixture is blocked in this workflow: escalate, do not retarget it or modify its harness. Follow existing HTTP mock/opt-in transport conventions, not new orchestration.
- Compile/test Linux inside Docker with a dedicated Linux cache, one cargo process at a time; never mount/share Windows `target` as Linux artifacts. Coordinate the cargo slot with the parent.
- Track fixture IDs during execution, including failure paths. Use bounded waits; do not automatically retry in a way that leaves previous fixtures alive. Keep the ledger current for recovery after a tool/session failure.

## 3. Release on success, failure or idle handback

1. Complete/cancel the bounded test operation and release consumers. A worker returns its cleanup inventory to the parent rather than tearing down resources still needed by parent/another worker/background validation. The parent performs final cleanup before final response once no actual next job is active. Explicit user keepalive is the only permission to retain otherwise unused runtime; record its scope and release condition.
2. Announce exact owned IDs/cluster and loss of volatile process memory. Standing user policy permits **reversible graceful `docker stop` of session-owned fixtures/Kind node containers** without another confirmation. Preserve persistent volumes and stopped containers. Not `docker kill`, deletion, prune, reset, `rm -f`, namespace/cluster removal, or automatic retries. Any actual removal needs the existing destructive guardrails and explicit scoped permission; conflicting hard prohibitions stay in force.
3. Compare final container inventory with BEFORE + ledger. Verify each owned fixture is removed by normal harness teardown or stopped; don't assume teardown/Ryuk succeeded (missing cleanup CLI, disabled Ryuk or process failure can leak). Report unknown/new unowned resources without adopting or stopping them. Handle a bounded stop failure by reporting the blocked cleanup, not escalating to forced deletion.
4. Stop owned unused Kind nodes gracefully; stop is not cluster deletion. Record any retained active lease/user keepalive and remaining resource IDs/reasons. Do not interrupt git operations or a host-only background publisher.

## 4. Return WSL RAM safely

1. Only after owned cleanup and proving no other active workload/lease, unknown ownership, or protected cluster would be affected: gracefully stop **Windows Docker Desktop**, then run `wsl --shutdown`. Explain that this stops **all WSL distributions** and loses their volatile memory. Check non-Docker WSL work too; keepalive forbids global shutdown while needed.
2. If that proof is unavailable or another workload exists, **do not globally shut down**. Report residual runtime/RAM and ask the parent/user for a scoped override; never silently affect protected contexts.
3. Verify Desktop process/state, WSL distribution states, and measured vmmem/available host RAM using Windows/WSL observations. Do not run Docker CLI after shutdown, as it may wake the daemon. Report measurements or INCONCLUSIVE, never unmeasured "zero RAM".

## Final return / recovery

Return: validation result; owned IDs and their final states; released/retained leases; Desktop/WSL state and measured RAM delta (or not measured); blockers and remaining resources. Recovery on the next task re-verifies the saved ledger before any cleanup; a stale ledger alone is not ownership proof.

These are agent procedures, **not an automated inactivity monitor**. Clean up before handing back/final response wherever possible; abrupt disconnect/kill cannot guarantee agent teardown. No idle monitor, hook, daemon or runner is installed by this skill. Do not create one or an operational ledger during a guidance-only task.
