# KUBERNETES — volume persistence lost during local-full rendering

> Historical discovery below. After explicit approval, PostgreSQL/Zot cutover and
> three-node CRI activation/pull were executed on the original cluster; see
> [execution results](../../../missions/20261101-local-full-cutover-results.md).

- Skill: KUBERNETES; target: local-full overlays and three existing Pending PVCs.
- Mode: focused diagnostic/correction; date: 2026-11-01; base: master e1607ae.
- Scope: source render and attested kind-aiforall-local-full only. No Secret GET,
  deletion, storage reset or workload rollout authorized by this report itself.
- Verification: live RO specialist + empirical offline Kustomize reproduction;
  parent source correction; live migration NOT RUN.

## P1

- [x] **#1 Persistence patches do not survive the actual renderer — source fixed**
  - Location: local-full/platform and apparatus patches/kustomization; renderer.
  - Evidence before fix: live/rendered postgres/zot data remain emptyDir, operator hostPath;
    three PVCs wait for first consumer. Redis is Bound and provisioner works.
    Kustomize v5.8.1 exits0 despite missing replacement effect. Root cause
    characterized empirically; no upstream issue attribution claimed.
  - Suggested action: JSON6902 name test + replace the individual volume element,
    preserve init/config/htpasswd; assert rendered PVC references and auxiliary mounts.
  - Fix applied: JSON6902 individual-element replacement and rendered guards.
    Base/GitLab/GitHub offline renders PASS, preserving init/config/htpasswd;
    mutation oracles reject emptyDir data or lost auxiliary volumes. Included in
    final batch61tests PASS32.406s. No live application implied.

- [x] **#2 Direct rollout would discard the existing emptyDir data — protected, then migrated**
  - Location: running postgres-7655db4dc4-bq2xf and zot-f4f58c85f-bq2fk.
  - Evidence: Running Pods mount emptyDir at /var/lib/postgresql/data and
    /var/lib/zot. Pending PVCs being empty does not prove existing data is empty.
  - Suggested action: preserve and verify data transfer before changing PodTemplates;
    require an explicit cutover plan/authorization, never use full deploy as a shortcut.
  - Fix applied: full deploy now refuses storage transitions before bootstrap,
    quiesce or apply; --confirm-quiesce cannot bypass it. No live cutover;
    [preservation plan](../../local-full-storage-cutover.md) requires approval.

## P2

- [x] **#3 Existing nodes lack the CRI registry hosts directory — activated after approval**
  - Location: containerd2.0.3 CRI images registry config_path empty on all three nodes.
  - Evidence: effective dump v3; separate transfer config_path also exists. CRI
    probe refused before client-key transfer or pull, not a TLS/data-plane result.
  - Suggested action: source Kind patch for future explicit creation; existing
    nodes need a separately authorized scoped activation/restart, not recreation.
  - Fix applied: source parser corrected and future Kind config prepared; no live
    containerd config or service restart executed.

Totals before approval: 1 source finding closed / 2 live operations open. Source validation and live cutover
qualification are separate; OpenBao local volatility and deferred auth E2E unchanged.

Execution update: all three findings resolved within their bounded scope. Original
PVC UIDs retained, auxiliary mounts untouched; exact effective CRI change validated
before progressive restarts. `crictl pull`/`inspecti` succeeded on all three original
nodes. This does not qualify the old OpenBao/operator installation or image refresh.

Independent verification on code62eeea5:6/6source checks PASS,0FAIL; actual offline
render113objects, auxiliary mounts preserved. Live identities and non-migrated
state confirmed through the original anchor; cutover/CRI pull still NOT RUN.
Existing containerd configuration can be enabled via a separately authorized
targeted change/restart; cluster recreation is neither necessary nor authorized.
Parent subsequently stopped its3original nodes and registered registry/helpers,
preserving the unrelated running local node. See the mission report for evidence.
