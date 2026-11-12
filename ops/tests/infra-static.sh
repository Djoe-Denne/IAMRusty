#!/usr/bin/env bash
# Offline only. Never apply, discover APIs, start a runtime, or execute E2E cases.
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"
PYTHON=${PYTHON:-python3}
command -v "$PYTHON" >/dev/null || { echo 'Python required (set PYTHON to its executable)' >&2; exit 1; }
# Windows hosts default to a legacy locale encoding for open()/stdin; UTF-8 mode
# keeps YAML sources (BOM-tolerant) and kubectl render pipes byte-faithful. No
# effect on Linux, where UTF-8 is already the default.
export PYTHONUTF8=1
command -v kubectl >/dev/null || { echo 'kubectl required for offline kustomize rendering' >&2; exit 1; }
HELM=${AIFORALL_HELM:-helm}
command -v "$HELM" >/dev/null || { echo 'Helm required for offline native Kind rendering' >&2; exit 1; }

scripts=0
while IFS= read -r -d '' script; do
  bash -n "$script"
  scripts=$((scripts + 1))
done < <(find ops -type f -name '*.sh' -print0)
[ "$scripts" -gt 0 ] || { echo 'No ops shell scripts found' >&2; exit 1; }

# Parse source YAML without printing document bodies (which may contain credentials).
"$PYTHON" - <<'PY'
from pathlib import Path
import yaml
class SourceLoader(yaml.SafeLoader):
    pass
def compose_value(loader, node):
    if isinstance(node, yaml.SequenceNode):
        return loader.construct_sequence(node)
    if isinstance(node, yaml.MappingNode):
        return loader.construct_mapping(node)
    return loader.construct_scalar(node)
for tag in ('!override', '!reset'):
    SourceLoader.add_constructor(tag, compose_value)
def is_helm_render_input(path):
    # Chart.yaml, not a directory named "charts", establishes this boundary.
    # Chart metadata and values remain ordinary YAML and must still be parsed.
    for parent in path.parents:
        if (parent / 'Chart.yaml').is_file():
            return path.relative_to(parent).parts[0] in {'templates', 'files'}
    return False
paths = [p for p in sorted(Path('ops').rglob('*.yaml')) + sorted(Path('ops').rglob('*.yml'))
         if not is_helm_render_input(p)]
paths.append(Path('.github/workflows/ci.yml'))
for path in paths:
    try:
        list(yaml.load_all(path.read_text(), Loader=SourceLoader))
    except yaml.YAMLError:
        raise SystemExit(f'Invalid YAML: {path}') from None
print(f'OK YAML syntax: {len(paths)} files')
PY

# Closed inventory: finding fewer overlays must never make the gate greener.
"$PYTHON" - <<'PY'
from pathlib import Path
expected = {
    'ops/deploy/apps/base',
    'ops/deploy/apps/overlays/kind',
    'ops/deploy/apps/overlays/kind-mesh',
    'ops/deploy/apps/overlays/kind-demo-monolith',
    'ops/deploy/p4',
    'ops/deploy/p4/knative',
    'ops/deploy/third-party/calico',
    'ops/deploy/third-party/calico/base',
}
actual = [p.parent.as_posix() for p in Path('ops/deploy').rglob('*')
          if p.name in {'kustomization.yaml', 'kustomization.yml'}]
if set(actual) != expected or len(actual) != len(expected):
    raise SystemExit('Deployment kustomization inventory differs from the eight declared renders')
PY
renders=0
while IFS= read -r -d '' manifest; do
  directory=$(dirname -- "$manifest")
  # No kubeconfig/context, no API server, no --dry-run=server.
  # Trusted repository overlays use cross-directory generator inputs (D contract).
  kubectl kustomize --load-restrictor LoadRestrictionsNone "$directory" | "$PYTHON" -c '
import sys, yaml
try:
    docs = [d for d in yaml.safe_load_all(sys.stdin) if d is not None]
except yaml.YAMLError:
    raise SystemExit("Invalid rendered YAML") from None
if not docs or any(not isinstance(d, dict) or not d.get("apiVersion") or not d.get("kind") or not d.get("metadata", {}).get("name") for d in docs):
    raise SystemExit("Missing Kubernetes resource identity in render")
print("OK rendered resource identities:", len(docs))
'
  printf 'OK offline render: %s\n' "$directory"
  renders=$((renders + 1))
done < <(find ops/deploy -type f \( -name kustomization.yaml -o -name kustomization.yml \) -print0)
[ "$renders" -eq 8 ] || { echo 'Expected exactly eight Kustomize renders (including pinned Calico)' >&2; exit 1; }
chart=ops/deploy/charts/kind-minimal
flags=(-f ops/deploy/build/targets.json)
for setting in applications.enabled=true buildTag=offline-static openbao.create=true postgres.create=true postgres.initialize=true redis.create=true openfga.create=true openfga.inspect=true openfga.initialize=true localstack.create=true localstack.initialize=true network.create=true zot.create=true; do
  flags+=(--set "$setting")
done
"$HELM" lint "$chart" --strict "${flags[@]}" >/dev/null
"$HELM" template kind-local "$chart" --namespace aiforall-local-full "${flags[@]}" | "$PYTHON" -c '
import sys, yaml
docs = [d for d in yaml.safe_load_all(sys.stdin) if d is not None]
if not docs or any(not isinstance(d, dict) or not d.get("apiVersion") or not d.get("kind") or not d.get("metadata", {}).get("name") for d in docs):
    raise SystemExit("Missing Kubernetes identity in Helm render")
identities = [(d["apiVersion"], d["kind"], d["metadata"].get("namespace", ""), d["metadata"]["name"]) for d in docs]
if len(identities) != len(set(identities)):
    raise SystemExit("Duplicate Kubernetes identity in Helm render")
for doc in docs:
    if doc["kind"] == "ConfigMap":
        for key, content in doc.get("data", {}).items():
            if key.endswith((".yaml", ".yml")):
                list(yaml.safe_load_all(content))
'
printf 'OK native Helm lint and offline render\n'
# Render/refusal oracles use synthetic public inputs only. This does not run k6,
# Chromium, the campaign parent or a Kubernetes client against a cluster.
"$PYTHON" -B -m unittest discover -s ops/tests/k6/tests -p test_native_campaign.py -k FixtureRenderTests -v
printf 'OK shell syntax: %s scripts; offline renders: %s\n' "$scripts" "$renders"
