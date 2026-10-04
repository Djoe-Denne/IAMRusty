# Local-full — source profile, parent execution only

Phase 2 implements an isolated **local** lab, not cloud/HA parity. No runtime,
build, cargo, IT or E2E was executed by D. All commands below except `render`
are for the parent **after integration and with the shared runtime lease**.
Legacy mesh/gold/J3 overlays remain separate; their host/hybrid limits remain.

## Dedicated cluster decision — explicit user authorization 2026-10-04

**S-11 supersedes the earlier lease format below:** see
[`cluster-anchor wiring`](../../../../../.cursor/review-briefings/20261004-package-d-cluster-anchor-wiring.md).
**User decision 2026-10-04:** the native Win32 mechanism is removed. Operational
helpers use the platform-neutral guard in `local_full_target.py`: fixed kubeconfig,
closed context allowlist, exact control-plane endpoint before any kubectl GET,
then UID validation. Positive CLI grammar and no-adoption checks remain.
This preflight guard does not claim native protection against concurrent file or
ancestor replacement. Runtime commands still require parent integration/final IT
and the shared runtime lease. Offline resource sources are unchanged.
All runtime/API helpers now require leasev2, exact created CP/node IDs and TCP6443
publication, fixed task-owned kubeconfig path/hash and prior kube-system UID.
Missing v1 anchors stop; never convert/adopt by probing an alias. New pending
creation is allowed only after final IT: bootstrap first with `-AfterFinalIT`,
then image manifest helper against the anchored lease, then deploy. No helper may
use ambient kubeconfig or a legacy lease to skip endpoint/UID binding.

All full-profile helpers exclusively target **cluster `aiforall-local-full`,
context `kind-aiforall-local-full`**. They never upgrade/recreate/restart the old
`aiforall-local` cluster. Legacy J2/J3/mesh retain `kind-aiforall-local` defaults;
protected or other contexts are not accepted. Shared PowerShell accepts precisely
these two cluster/context names, defaulting to legacy; full bootstrap selects the
new name explicitly and rejects an old-context lease.

Kind config: one control-plane + two workers, unchanged pinned node image;
API loopback `127.0.0.1`, random port (`apiServerPort: 0`), never old 60787.
HTTP mapping **127.0.0.1:18080 → control-plane:30080 → Envoy Service:10000**.
Only local-full patches the Service NodePort; legacy service/config unchanged.
No 18443 TLS edge listener is invented (Envoy's current ingress listener is HTTP).
This mapping is source-validated only: host port occupancy has NOT been inspected.
Future create first binds/releases loopback 18080; busy => STOP, never kill its owner.
The API endpoint may be inventoried later with explicit full-context `config view`
metadata; full `inventory` lists workloads/PVCs/Jobs/policies without Secret values.

Before create, parent lease must specify `context`, `cluster`, `nodeContainerIds`
(empty for an absent cluster pending actual inventory). Bootstrap records BEFORE
full container IDs and creation attempt, then exact post-create new IDs/state by
action delta, including partial failure. Labels/names alone never prove ownership.
`resourceStates` and created-resource entries carry the full context/cluster.
Reuse requires explicit matching leased IDs, running pinned nodes, exact 3-node
topology, loopback host mapping and compatible CNI/CIDR. Unknown/stopped/conflicting
nodes => STOP without deletion/replacement. No claim the new cluster exists yet.

## Included / simulated / absent

- Real in-Kind standalone IAM, Hive, Telegraph, Manifesto + Lazaret, HTTP/mTLS
  mesh Envoy/ext-authz, real PostgreSQL migrations/outbox, OpenFGA store/model,
  sentinel-sync using its PostgreSQL idempotency ledger, SQS protocol via
  LocalStack, Redis KV, OpenBao Kubernetes auth/Transit and OCI Zot.
- Local simulations: LocalStack's SQS backend, existing component catalog stub,
  MailHog SMTP sink, development OpenBao memory backend, local credentials/test
  PEM. These are not production secret/registry/mail guarantees.
- External OAuth connectors are disabled (`connectors=[]`), not falsely called
  in-cluster simulations. Vendor OAuth/PKCE end-to-end needs E/parent's existing
  connector fixture/config/images. No GKE, Flux, HSM, cloud HA or full ADR-0602
  assertion. `postgres`, `zot`, `redis`, `admission-store` use local-path PVCs;
  LocalStack queue contents and OpenBao DEV state remain volatile.
- Four platform services scale to two worker-spread replicas after serial boot
  migrations. Lazaret remains one replica: its existing enrollment state is
  process-local. Single-node PostgreSQL/registry storage is persistence, not HA.

## Offline render and parent entrypoint

Prerequisites: existing `kubectl` (built-in Kustomize), Python + PyYAML. Render
never calls the API or Docker and prints only kind/name metadata:

```powershell
python -B ops/deploy/deploy-local-full.py render --output-dir <NEW-directory>
# Or split foundation/dependencies/bootstrap/applications:
python -B ops/deploy/render-local-full.py --output-dir <NEW-empty-directory>
```

Output files include **known local-development** credentials/config; do not
publish them as logs. Prepared private certificate inputs are never printed.
`LoadRestrictionsNone` reads only trusted repository sources; no remote bases.

The parent builds Linux artifacts once in Docker using the existing Dockerfiles;
sentinel's runtime wrapper is `ops/deploy/build/Dockerfile.worker` (default binary
`sentinel-sync`, `BUILD_IMAGE` the existing artifacts image). Lazaret uses its
existing Dockerfile. Nothing in this entrypoint implicitly builds/pulls/starts DD.

```powershell
python -B ops/deploy/local-full-images.py --lease <parent.json> --output <NEW-images.json>
# prepare-local-full-certs.sh requires openssl + explicit NEW LOCAL_FULL_CERT_DIR;
# run only under the parent lease, not as an implicit deployment action.
python -B ops/deploy/deploy-local-full.py deploy --lease <parent.json> --image-manifest <images.json> --cert-directory <prepared-certs>
python -B ops/deploy/deploy-local-full.py inventory --lease <parent.json>
```

Image manifest maps all nine exact source image references to full
`:sha256-<64hex image ID>` tags, validated against Docker before Kind import.
Generated ConfigMap names + full configuration/certificate checksum change the
PodTemplate. The render includes no `host.docker.internal`/host monolith path.

Lease requires `context=kind-aiforall-local-full`, `cluster=aiforall-local-full`, exact `nodeContainerIds`, and for
reuse exact `namespaceUids`, `ownedResources` (kind/name/namespace/uid), plus
`clusterResourceUids` for pre-existing shared CRD/cluster RBAC. Existing incompatible
topology/node image/CNI/schema or unknown ownership **STOPs**: no adopt/restart,
delete/replace/prune. Existing owned deployments require `--confirm-quiesce`;
apps stop before serial migrations, and failure restores prior replica counts.
It is not database/schema rollback. Completed bootstrap Jobs are retained under
unique run names. Inventory them before quota exhaustion; no automatic deletion.
Deployment/storage/recovery share `.local-full-lock` beside the lease. A stale lock requires
parent ownership review, never automatic reclamation.

## Namespace/client source handoff — integrated by other owners, not runtime proof

1. `services/Lazaret/configuration/src/lib.rs`: now supplies
   `PluginHopConfig.namespace` with legacy default `apparatus-plugins`; when DNS
   formula is enabled, validate nonempty DNS-1123 label (1..63 chars, lowercase
   alnum/hyphen, alnum ends). No implicit blank fallback or arbitrary URL.
2. `services/Lazaret/application/src/invoke.rs`: preserves existing
   `DigestDnsPluginLocator`/`plugin_dns_endpoint` legacy API and now supplies
   `NamespacedDigestDnsPluginLocator::try_new(namespace)`. Endpoint remains
   `http://plugin-{first32 digest hex}.{namespace}.svc:8080`; retain digest-based
   binding/instance checks and metadata. Never use binding ID as release identity.
3. `services/Lazaret/setup/src/app.rs::plugin_endpoint_locator`: now constructs the
   namespace-aware locator from the validated config and propagates validation error,
   no silent legacy namespace fallback. Exports in application/shared `lib.rs`
   remain parent-owned. D's operator has `ControllerNamespaces::try_new`, getters,
   `with_namespaces`; `connect_default` reads both `APPARATUS_SYSTEM_NAMESPACE`
   and `APPARATUS_PLUGINS_NAMESPACE` together (neither => legacy defaults).
   Parent exports this API if needed; D did not edit shared exports.
4. `crates/apparatus-reference-kv/src/bin/reference-kv-http.rs` now reads
   `LAZARET_CA_CERT_PATH` PEM once for verified HTTPS, preserving mTLS, no redirects,
   15s timeout and 64KiB response bound. No additional shared workload-client adapter
   exists/was invented. Application/setup/client source was not co-edited by D.

### Isolated plugin public CA projection

Parent deploy populates **only `ca.crt`** in Secret `lazaret-workload-ca` in
`aiforall-local-plugins`, from the prepared workload CA bundle. No `ca.key`,
`server.key`, transport keys or other workloads' credentials are copied. The
operator does not GET the Secret or mount any signing keys: kubelet projects the
single explicit key via `SecretVolumeSource.items` into generated plugin Pods,
read-only mode 0444 at `/var/run/lazaret-ca/ca.crt`.

Only `--isolated-plugins` configures operator `APPARATUS_PLUGIN_CA_SECRET` and
`APPARATUS_PLUGIN_CA_SHA256` together. Both absent => legacy behavior; partial or
invalid secret-name/hash => refusal. Generated workload env sets
`LAZARET_CA_CERT_PATH=/var/run/lazaret-ca/ca.crt` plus a public CA hash for drift
matching. A reserved-volume/mount collision refuses the artifact rather than
overwriting it. Existing Pod reuse requires the same projection/path/hash;
rotation cannot retain an old read-once CA client under the same digest Pod.
Enrollment URL remains
`https://lazaret.aiforall-local-full.svc.cluster.local:8080/lazaret/enroll`, covered
by the prepared server certificate's DNS SAN. No CA-fetch egress or NP exception.

Basic lab explicitly uses `use_dns_formula=false` and does **not** claim plugin
invoke closure. With integrated sources, opt in with `--isolated-plugins
--namespace-wiring-confirmed`. The confirmation is parent acceptance, not an
runtime capability test. The entrypoint first checks concrete namespace/client
source markers and records SHA-256 of all four source files in the parent lease;
missing source wiring fails even with the acknowledgement. Source presence is
not proof the immutable images contain that code: parent must match these hashes
to built artifacts and final tests. Admission/signing runs retain existing P4 gates;
no bypass for local-full. Plugin artifacts/signing fixture seeding remains parent/E.

## E helper CLI / probe contracts

- Namespace targets: `aiforall-local-full`, `aiforall-local-apparatus`,
  `aiforall-local-plugins`; workload selector `app.kubernetes.io/name=<service>`.
  Pod templates carry `aiforall.dev/profile=local-full`.
- Platform readiness paths `/iam/ready`, `/hive/ready`, `/telegraph/ready`,
  `/manifesto/ready` on HTTP 8080; Lazaret `/lazaret/ready` HTTPS 8080. Sentinel
  exec probes check process/dependency availability, **not projection convergence**.
  Operator has no fake listener/probe; assert watch reconciliation, not HTTP.
- `default-deny`/`allow-dns` in each namespace. Explicit same-namespace data hops,
  Envoy→ext-authz:8090 and services:8443, Lazaret↔plugin:8080. Plugin selectors
  require `apparatus.aiforall.dev/project`, `/binding`, `app.kubernetes.io/instance`.
  No arbitrary external egress. Parent configures the actual control-plane IP for
  API egress before bootstrap Jobs. `local-full-client` pods may reach Envoy:10000
  only (plus DNS); an unlabeled negative probe pod must be denied. A backup
  exec probe needs no network exception. kubectl port-forward is not NP evidence.
- `python -B ops/deploy/local-full-storage.py backup --lease <lease> --archive
  <NEW-file> --volume postgres --database manifesto_dev`; use `zot` or `admission`
  with `--confirm-quiesce` for volume snapshots. Quiesce DB writers externally;
  relation-count fingerprints detect count changes but are **not** a consistent
  multi-service/global snapshot proof. Native pg_dump provides a DB snapshot.
- `... restore --lease <lease> --archive <file> --target-namespace
  aiforall-local-restore-<unique> --target-database manifesto_restored
  --confirm-restore`. Existing target namespace, same DB/PVC/PV, nonempty database,
  checksum mismatch or unsafe tar entry => fail. No drop/clean/wipe/retarget. New
  PVC remains separate and is never wired over a live workload. Compare restored
  relation counts plus E's domain invariants; count hash alone does not prove data
  equality. Tar helpers reject symlinks/special/path traversal; portable volumes
  containing links need an explicit different safe policy, not a bypass.
- E additionally covers HTTPS wrong CA, wrong SAN, missing CA file, CA projection
  containing no private keys, reserved-mount conflicts and CA-hash rollout/reuse.
  Real artifact admission/signing/seed fixtures remain required before invoking.
- Storage helper records exact created Pod/PVC/namespace UIDs in the lease. Backup
  staging process stops, source replica count restores in `finally`; restore Pod
  has a 900s deadline. Parent releases still-active owned fixture Pods after
  assertions/failure. PVCs/archives retained, no automated resource deletion.
- `python -B ops/deploy/local-full-node-recovery.py plan|interrupt|recover
  --lease <lease> --node-id <FULL-ID>`; mutation adds
  `--confirm-node-interruption`. Only a running, uncordoned, explicitly leased
  local Kind **worker** may be cordoned/stopped. Recover requires its persisted
  interruption record; restarts the same ID, waits Ready, then uncordons. No drain,
  delete, rebuild or cluster replacement. Local-path PVC stays on that worker;
  expect outage until recovery, not transparent failover. E verifies topology,
  surviving service readiness, recovered persisted rows/OCI/store contents.

## Evidence and open finding closure

Offline render succeeds with 112 objects (93 foundation, 8 dependencies, 2 Jobs,
9 application Deployments). This is source/render evidence only; parent still
owns compilation, durable acceptance coverage and final integrated IT/E2E.
I4/I8/I9 source provisions now exist; no finding is runtime-closed. Other legacy
profile policies/budgets/pins and IaC CI remain pending per Package D checkpoint.
