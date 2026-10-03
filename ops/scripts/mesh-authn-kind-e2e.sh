#!/usr/bin/env bash
# Replay the mesh AuthN contract on kind-aiforall-local.
# Does not delete the cluster, a namespace, or a volume.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# Bounded source-layout regression check: no cluster access or secret contents.
# Fail closed if the known Secret projection shape changes; review it explicitly.
check_app_cert_projections() {
  awk '
    function finish() {
      if (app != "") {
        if (bundles != 1) bad = 1
        seen[app]++
      }
    }
    boundary {
      # Scan the whole application volume until the real YAML scope
      # boundary: the document separator, or a non-blank line dedented
      # below the certs volume entry (<9 leading spaces). Blank lines do
      # not close the scope, so an extra projected key cannot hide after
      # one; any non-blank line still inside the scope fails closed.
      if ($0 ~ /^[[:space:]]*\r?$/) {
        # blank line: still inside the scanned scope
      } else if ($0 ~ /^---\r?$/ || $0 !~ /^         /) {
        boundary = 0
      } else {
        bad = 1
      }
    }
    /^---\r?$/ { finish(); app = ""; bundles = 0; kind = "" }
    /^kind:/ { kind = $2; sub(/\r$/, "", kind) }
    /^  name: (iam|hive|telegraph|manifesto)\r?$/ && kind == "Deployment" {
      app = $2; sub(/\r$/, "", app)
    }
    /secretName: platform-mesh-certs/ && app != "" {
      bundles++
      expected[1] = "            items:"
      expected[2] = "              - key: ca.crt"
      expected[3] = "                path: ca.crt"
      expected[4] = "              - key: " app "-service.crt"
      expected[5] = "                path: " app "-service.crt"
      expected[6] = "              - key: " app "-service.key"
      expected[7] = "                path: " app "-service.key"
      for (i = 1; i <= 7; i++) {
        if ((getline line) <= 0) { bad = 1; break }
        sub(/\r$/, "", line)
        if (line != expected[i]) bad = 1
      }
      # No additional projected identity or path may follow the allowlist.
      boundary = 1
    }
    END {
      finish()
      if (seen["iam"] != 1 || seen["hive"] != 1 ||
          seen["telegraph"] != 1 || seen["manifesto"] != 1) bad = 1
      exit bad ? 1 : 0
    }
  ' "$ROOT/ops/deploy/apps/overlays/kind-mesh/platform-services.yaml" || {
    echo "FAIL application certificate projections: expected only CA + own cert/key" >&2
    return 1
  }
  echo "OK application certificate projections (CA + own cert/key only)"
}
check_app_cert_projections
if [ "${1:-}" = "--static-only" ]; then
  exit 0
fi
hostpath() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s' "$1"
  fi
}
CTX="kind-aiforall-local"
FORBIDDEN="kind-apparatus-p4-it"
current="$(kubectl config current-context)"
if [ "$current" = "$FORBIDDEN" ] || [ "$current" != "$CTX" ]; then
  echo "STOP contexte kubectl: $current (attendu $CTX)" >&2
  exit 1
fi

stamp="$(date +%Y%m%d%H%M%S)"
ns_platform="aiforall-platform"
ns_gateway="aiforall-gateway"

kubectl --context "$CTX" -n "$ns_gateway" create configmap "mesh-kind-e2e-cases-${stamp}" \
  --from-file=cases.sh="$(hostpath "$ROOT/ops/scripts/mesh-authn-kind-e2e-cases.sh")" \
  --dry-run=client -o yaml | kubectl --context "$CTX" apply -f -

kubectl --context "$CTX" -n "$ns_gateway" create configmap "mesh-kind-fga-model-${stamp}" \
  --from-file=model.json="$(hostpath "$ROOT/ops/openfga/model.json")" \
  --dry-run=client -o yaml | kubectl --context "$CTX" apply -f -

cat <<EOF | kubectl --context "$CTX" apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: mesh-fga-boot-${stamp}
  namespace: ${ns_gateway}
  labels:
    app.kubernetes.io/name: mesh-authn-e2e
    app.kubernetes.io/part-of: aiforall
    app.kubernetes.io/component: mesh-authn
spec:
  restartPolicy: Never
  automountServiceAccountToken: false
  containers:
    - name: boot
      image: alpine:3.20
      imagePullPolicy: IfNotPresent
      command: ["sh", "-c"]
      args:
        - |
          set -eu
          apk add --no-cache curl jq >/dev/null
          deadline=\$(( \$(date +%s) + 120 ))
          while ! curl -fsS --max-time 5 http://openfga.${ns_platform}.svc.cluster.local:8080/healthz >/dev/null; do
            if [ "\$(date +%s)" -ge "\$deadline" ]; then
              echo "FAIL OpenFGA healthz" >&2
              exit 1
            fi
            sleep 2
          done
          store_id=\$(curl -sS -X POST http://openfga.${ns_platform}.svc.cluster.local:8080/stores \
            -H "content-type: application/json" \
            -d '{"name":"aiforall-kind-mesh"}' | jq -r .id)
          [ -n "\$store_id" ] && [ "\$store_id" != "null" ]
          model_id=\$(curl -sS -X POST "http://openfga.${ns_platform}.svc.cluster.local:8080/stores/\${store_id}/authorization-models" \
            -H "content-type: application/json" \
            --data-binary @/model.json | jq -r .authorization_model_id)
          [ -n "\$model_id" ] && [ "\$model_id" != "null" ]
          printf '%s %s\n' "\$store_id" "\$model_id"
      volumeMounts:
        - name: model
          mountPath: /model.json
          subPath: model.json
  volumes:
    - name: model
      configMap:
        name: mesh-kind-fga-model-${stamp}
EOF

kubectl --context "$CTX" -n "$ns_gateway" wait --for=jsonpath='{.status.phase}'=Succeeded \
  "pod/mesh-fga-boot-${stamp}" --timeout=180s
fga_line="$(kubectl --context "$CTX" -n "$ns_gateway" logs "mesh-fga-boot-${stamp}" | tail -n 1)"
fga_store="${fga_line%% *}"
fga_model="${fga_line##* }"
echo "OpenFGA store ${fga_store} model ${fga_model}"

kubectl --context "$CTX" -n "$ns_platform" set env deployment/hive \
  "HIVE_OPENFGA__STORE_ID=${fga_store}" \
  "HIVE_OPENFGA__AUTHORIZATION_MODEL_ID=${fga_model}"
kubectl --context "$CTX" -n "$ns_platform" rollout status deployment/hive --timeout=240s

cat <<EOF | kubectl --context "$CTX" apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: mesh-authn-kind-e2e-${stamp}
  namespace: ${ns_gateway}
  labels:
    app.kubernetes.io/name: mesh-authn-e2e
    app.kubernetes.io/part-of: aiforall
    app.kubernetes.io/component: mesh-authn
spec:
  restartPolicy: Never
  automountServiceAccountToken: false
  containers:
    - name: e2e
      image: alpine:3.20
      imagePullPolicy: IfNotPresent
      command: ["sh", "/cases.sh"]
      env:
        - name: OPENFGA_STORE_ID
          value: "${fga_store}"
      volumeMounts:
        - name: cases
          mountPath: /cases.sh
          subPath: cases.sh
        - name: certs
          mountPath: /certs
          readOnly: true
  volumes:
    - name: cases
      configMap:
        name: mesh-kind-e2e-cases-${stamp}
    - name: certs
      secret:
        secretName: platform-mesh-certs
EOF

deadline=$((SECONDS + 300))
while true; do
  phase="$(kubectl --context "$CTX" -n "$ns_gateway" get "pod/mesh-authn-kind-e2e-${stamp}" -o jsonpath='{.status.phase}')"
  case "$phase" in
    Succeeded)
      break
      ;;
    Failed)
      kubectl --context "$CTX" -n "$ns_gateway" logs "mesh-authn-kind-e2e-${stamp}" || true
      exit 1
      ;;
  esac
  if [ "$SECONDS" -ge "$deadline" ]; then
    kubectl --context "$CTX" -n "$ns_gateway" logs "mesh-authn-kind-e2e-${stamp}" || true
    echo "FAIL e2e pod phase=$phase" >&2
    exit 1
  fi
  sleep 2
done
kubectl --context "$CTX" -n "$ns_gateway" logs "mesh-authn-kind-e2e-${stamp}"
