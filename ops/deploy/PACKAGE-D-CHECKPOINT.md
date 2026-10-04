# Package D checkpoint — 2026-10-03

**Partial source changes, static evidence only. Not finding closure or a delivered local-full lab.** Baseline execution HEAD: `5348a63`. No runtime, cargo, build, deployment, live Secret reads or IT/E2E execution by this worker.

## Source changes and static evidence

- I1: marker-based repository root discovery shared by M1/M2/M3/J2/J3/mesh/gold helpers; fixed relocated certificate-script mounts and reference-KV staging sources. `ops/scripts/validate-deploy-static.ps1` passes PowerShell parsing, root discovery and the listed nominal paths.
- I2/I3: J2 no longer deletes/recreates clusters or IPPools. Shared bootstrap used by M2/J2/J3/mesh: explicit local context, parent lease with exact node IDs, create-if-absent only, no stopped-node restart, pinned Calico/pool before first apply, bounded Calico/CoreDNS/node waits. Existing CNI/pool/node-image/topology conflicts stop without replacement. Runtime behavior remains unverified.
- I4: separate opt-in multi-node Kind config (1 control-plane + 2 workers), pinned node image. No namespace-isolated local-full workload overlay yet.
- I5: mesh owns its two default-deny policies; controller plugin RBAC narrowed to actual pod/service verbs. Other profiles and allow/deny probes still need completion.
- I6: mesh Rust services have periodic prefixed `/ready` probes plus existing startup health checks and resource budgets; PG/FGA/Envoy/ext-authz and operator containers have budgets. Plugin PodSpec gets budgets and startup/readiness TCP probes on its existing invoke-port contract (8080), with an internal unit test authored but not executed. TCP readiness is not admission/authorization health. Operator is a watch process with no listener; no fake HTTP probe added.
- I7: generated mesh ConfigMap names retain hashes; J2 and mesh local app images get full image-ID content tags, config/image identity PodTemplate annotations and rollout waits. Mesh inline Envoy config gets a generated checksum. Other profiles/Zot/cert rotation still need equivalent identity handling.
- I10: Kind/mesh Envoy/FGA/PG tags and digests checked against public Docker Hub metadata; README CIDR corrected to 10.244.0.0/16. Remaining Compose/profile pins and IaC CI are not finished.
- C2: Compose and Kind Envoy method/path public allowlists match the observed IAM router; static parity and 13 allowed pairs/wrong-method/protected-route denial checks pass. `mesh/public-routes.md` records the matrix; A must reconfirm after phase 2, E owns final E2E.

Other static checks: YAML syntax for 49 deployment YAML files passes (Compose `!override` syntax retained); `rustfmt --check --edition 2021 workers/apparatus-operator/src/controller.rs` and scoped `git diff --check` pass. Standalone Kustomize is absent: **no rendered-resource validation claim**. No kubectl command was executed.

## Required continuation before final tests

1. Complete isolated `aiforall-local-*` namespace profiles, labels, residue inventory and safe exclusivity; preserve legacy demos. The helper intentionally blocks an existing incompatible cluster rather than replacing it. Parent needs a scoped ownership/topology resolution if the existing lab cannot host local-full.
2. I8: implement and wire the representative 4+1 in-Kind path with sentinel-sync, queue/outbox projection via LocalStack SQS, Redis KV, local OpenBao, optional Zot/catalog and simulated enabled vendors. Current mesh disables queues, and gold/J3 use host collaborators: **neither is local-full**. No cloud/HSM/HA/0602 claim.
3. I9: add persistent local-path PG/Zot/admission-store PVC variants; safe backup/restore to a distinct empty target/PVC and consent-gated owned-node recovery helpers. No restore or failure scenario authored/executed yet.
4. Finish budgets/probes in legacy/base profiles and local-full quotas/PDB/topology; make every profile's policies autonomous and plugin selectors compatible with actual operator labels.
5. Parametrize operator/locator namespace wiring under the architectural interface without changing digest-based instance keys. Shared exports/setup/Cargo remain integration-owned; ask the parent for that wiring, not drive-by edits.
6. E owns durable route/NetworkPolicy/rollout/failure/restore tests and static IaC CI. Parent owns the single cargo/runtime lease and all final IT + E2E after all packages are integrated.

Owned fixture/container IDs: **none**. No runtime started/restarted, so no worker cleanup action or RAM measurement applies. Parent runtime/leases remain untouched.

## Phase 2 continuation — 2026-10-04 (after server restart)

The phase-1 sections above are historical evidence, not a second implementation
pass. HEAD remains `5348a63`; external/shared owner edits and `.tmp-layout-rewrite/`
were not reverted. Current detailed handoff:
[`local-full/README.md`](apps/overlays/local-full/README.md).

- I4/I8 source: new isolated `aiforall-local-full`, `aiforall-local-apparatus`,
  `aiforall-local-plugins` overlay. Real 4+1 standalone paths, in-Kind PG/FGA,
  sentinel/Postgres ledger, LocalStack SQS, Redis, OpenBao DEV, Zot and existing
  catalog/MailHog simulations. No host collaborator DNS in rendered resources.
  Four platform services get 2 replicas/worker topology spread; Lazaret remains
  one replica with its existing process-local enrollment limitation. OAuth
  vendors are explicitly disabled, not represented as a completed simulation.
- I5/I6/I7 source: autonomous default-deny/explicit hops in all three namespaces,
  quotas/LimitRanges/PDBs, prefixed readiness, full image-ID content manifest,
  hashed configs + certificate/config checksum PodTemplates. Sentinel probes are
  dependency availability, not event convergence; operator remains a watch-only
  process. Existing legacy profiles still need their own remaining improvements.
- I9 source: PG/Zot/Redis/admission-store local-path PVCs, binary pg_dump + safe
  quiesced volume backup helpers, checksum-checked restore to a NEW distinct
  namespace/DB/PVC/PV, and consent-gated exact-owned-worker stop/recover. No
  destructive resets or automatic removal. Relation counts are supplemental
  evidence, not domain/data equality; node-local persistence is not HA.
- Entry point `deploy-local-full.py`: default offline render via just recipe;
  actual deploy requires parent lease, prebuilt immutable image manifest, separate
  prepared CAs. Parent UID ownership and cluster schema conflicts STOP. Bootstrap
  is non-destructive; bounded waits, active Job refusal, operation lock, consent
  before re-deploy quiescence, prior replica rollback on failure. No build/start
  Docker Desktop or key generation hidden inside deployment.
- D-owned operator: typed validated complete distinct namespace pair, environment
  mapping in `connect_default`, legacy defaults preserved, digest identity
  unchanged; authored internal test remains unexecuted. Parent exports and
  Lazaret config/namespace-aware locator/setup/client CA wiring remain required,
  with exact files/API semantics in the local-full README. Basic profile disables
  DNS formula; isolated plugin invoke is opt-in and **not closed**.
- C2 reconciled with A's `http/src/public_routes.rs`: 13 declared pairs + five
  implicit HEAD pairs, no wildcard methods, relink/link/S2S/echo remain protected.
  Final A2 router confirmation and E end-to-end route proof still required.
- Local dependency pins now include LocalStack/OpenBao/Redis/Python/Node/MailHog
  from public metadata/manifest digests; no image pull occurred. Zot uses a pinned
  local image-ID at deploy. Remaining global/legacy pins and IaC CI are open.

**Still not delivered/runtime-closed:** no Docker/node/API/deploy/cargo/IT/E2E
execution by D. New bootstrap/storage/recovery helpers are source-only, require
parent review and final integrated proofs. No owned fixtures or runtime cleanup.
Static render/syntax evidence is appended below after final checks.

### Phase 2 final static evidence (no API calls)

- `python -B ops/deploy/deploy-local-full.py render`: **PASS**, 111 resources:
  3 namespaces, 17 Deployments, 4 PVCs, 4 PDBs, 29 NetworkPolicies, 2 bootstrap
  Jobs; metadata only printed. No actual immutable image manifest supplied at
  static time; actual deploy requires one.
- Built-in `kubectl --context kind-aiforall-local kustomize --load-restrictor
  LoadRestrictionsNone ops/deploy/apps/overlays/local-full`: **PASS** in the
  renderer and direct offline diagnostic. No server access, generated Secret
  values not printed. Split render: foundation 92/dependencies 8/bootstrap
  2/applications 9. Python-render invariants find no host/hybrid DNS, namespace
  escape or container missing resource budgets; full CPU limit total 15.4 <= 20.
- Python `ast.parse` on six new helper sources and `tomllib.loads` on three
  local-full config files: **PASS** (syntax only, no unit/runtime execution).
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
  ops/scripts/validate-deploy-static.ps1`: **PASS** with surviving/new PS sources.
- `rustfmt.exe --check --edition 2021
  workers/apparatus-operator/src/controller.rs`: **PASS**; no Rust compilation.
- `bash -n ops/scripts/prepare-local-full-certs.sh` and `bash -n
  ops/deploy/apps/overlays/local-full/helpers/seed-openbao.sh`: **PASS**.
- `git diff --check -- ops/deploy ops/scripts/prepare-local-full-certs.sh
  workers/apparatus-operator/src/controller.rs justfile`: **PASS**.
- Python/PyYAML static public-route matcher comparison: both Envoy public match
  structures identical, 5 match groups with 2 explicitly anchored GET|HEAD
  groups, no echo/relink exemption: **PASS**. Final routing behavior is E's proof.

The first render diagnostic exposed missing original namespaces on strategic
patch selectors; corrected and render passed. Two local scratch JS/Python
syntax errors were corrected; not runtime/infrastructure failure or tests.
No runtime resources, fixture IDs, Secret reads, builds, commits or deletions.

### C2 final architectural delta — section 9

The earlier blanket “relink-callback remains Bearer protected” claim above is
superseded ONLY for GET/HEAD on the exact github/gitlab `/relink-callback` paths.
Compose/Kind now exempt these four pairs from access JWT, subject to the integrated
A2 signed-state/browser-DB Relink consume guard before any effect. No deployment
before that guard and callback-only SDK change are matched. Counts are 22 symbolic
pairs / 26 concrete pairs over both enabled providers; local-full registry stays
empty and its generator removes both new callback exemptions (zero new pairs).
All START/account/link/S2S/echo protections remain. No D2 provisions were redone.

Static C2 delta validation: Python/PyYAML comparison **PASS** (Compose/Kind 7
identical match groups, exactly 4 added method/path pairs; development github/gitlab
registry redirects confirmed); local-full offline render **PASS**, 111 objects,
zero added relink exemptions for its empty registry. Scoped `git diff --check`
**PASS**. No API/runtime/deployment or handler source changes performed.

### Final public workload-CA delta after namespace/client handoff

Namespace/client sources now supplied by their owners: `PluginHopConfig.namespace`,
`NamespacedDigestDnsPluginLocator::try_new`, validated setup, and reference-KV's
`LAZARET_CA_CERT_PATH` verified HTTPS client. D changed only controller, local-full
entrypoint/plugin manifest and documentation; no application/shared-export edits.

Isolated plugin deploy now creates a separate **public-only** plugin-namespace
Secret `lazaret-workload-ca` from prepared `identity["ca.crt"]`. The operator's
optional `APPARATUS_PLUGIN_CA_SECRET`/`APPARATUS_PLUGIN_CA_SHA256` pair projects
only key `ca.crt` read-only mode 0444 and injects
`LAZARET_CA_CERT_PATH=/var/run/lazaret-ca/ca.crt`. No private CA/server/transport
keys projected; no CA-fetch network rule, signature/admission/identity bypass.
Reserved mount collisions fail; existing Pod reuse checks projection/path/public
hash so read-once CA drift is not silently accepted. Legacy pair-absent behavior
and base `use_dns_formula=false` remain. DNS true + exact local plugin namespace
are enabled only with isolated opt-in.

The acknowledgement flag no longer suffices by itself: isolated entrypoint checks
the four actual namespace/client source markers and records source SHA-256 in the
lease. Parent still must match source to built image artifacts and ratify final
tests; source detection is not runtime attestation or delivered invoke closure.
No additional workload adapter was found or invented. New internal CA projection
test authored, not compiled/executed. No new shared exports required by this delta.

Offline renderer: **PASS**, 112 objects (one additional public-CA Secret placeholder,
93 foundation/8 dependencies/2 bootstrap/9 applications). Python AST and targeted
source/schema inspections **PASS**; no Secret values printed. E gets wrong-CA,
wrong-SAN, missing-CA, CA-only projection, mount-conflict and CA-rotation proofs.
No runtime, cargo, API, deployment or fixture ownership (IDs remain none).

### Dedicated full cluster arbitration — 2026-10-04 (source-only)

User explicitly chose a NEW `aiforall-local-full` cluster / `kind-aiforall-local-full`
context. This supersedes old-target references in earlier checkpoint history for
full helpers ONLY. Shared PowerShell now accepts exactly legacy/full names, with
legacy default; full bootstrap passes the new name and strict context/cluster lease.
All full Python deployment/import/render/images/backup/restore/recovery paths use
only the full target. J2/J3/mesh and their single-CP config are unchanged.

Full Kind source retains pinned node image and exact 1+2 topology; API loopback
random port (0), one mapping 127.0.0.1:18080 -> CP:30080 -> Envoy Service:10000.
No additional HTTPS edge listener invented. Future creation probes port occupancy
and STOPs if busy (no kill). Reuse verifies exact leased running node IDs, topology,
host mapping and CNI. No unknown node/cluster upgrade or deletion. Existence check
is filtered to full cluster including stopped nodes; actual absence is NOT claimed.

Create action persists BEFORE IDs, attempt metadata, exact newly created IDs/state
including partial failure; known records include context/cluster/resourceStates.
Inventory reports explicit full context/API/HTTP endpoints and leased IDs, without
Secret contents. No runtime inventory/start/API/build/Cargo/IT/E2E performed here;
old/protected clusters untouched. Mechanical policy metadata belongs to its owner.
Provisioned resource count remains 112 (only full Envoy Service type/NodePort changes).

Dedicated-target static validation **PASS**: Python AST/full-only literals in all
five Python helpers; offline Kustomize render 112 objects; exact 1+2 Kind roles,
loopback API port 0, host18080/node30080/Envoy10000 mapping; PowerShell parser/root/
path validator; scoped `git diff --check`. Host-port availability, cluster existence,
API endpoint and actual node/container state were NOT observed. No Docker/Kind
runtime or kubectl API command executed; no runtime resources created.
