#!/usr/bin/env bash
# Offline only. Never apply, discover APIs, start a runtime, or execute E2E cases.
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"
PYTHON=${PYTHON:-python3}
command -v "$PYTHON" >/dev/null || { echo 'Python required (set PYTHON to its executable)' >&2; exit 1; }
command -v kubectl >/dev/null || { echo 'kubectl required for offline kustomize rendering' >&2; exit 1; }

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
paths = sorted(Path('ops').rglob('*.yaml')) + sorted(Path('ops').rglob('*.yml'))
paths.append(Path('.github/workflows/ci.yml'))
for path in paths:
    try:
        list(yaml.load_all(path.read_text(), Loader=SourceLoader))
    except yaml.YAMLError:
        raise SystemExit(f'Invalid YAML: {path}') from None
print(f'OK YAML syntax: {len(paths)} files')
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
[ "$renders" -gt 0 ] || { echo 'No deployment kustomizations found' >&2; exit 1; }
printf 'OK shell syntax: %s scripts; offline renders: %s\n' "$scripts" "$renders"
