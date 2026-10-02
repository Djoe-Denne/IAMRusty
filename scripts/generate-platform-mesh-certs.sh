#!/bin/sh
# Generate-if-absent platform mesh CA + per-service leaves for Hive/IAM/Telegraph/Manifesto/GitHub Connect/GitLab Connect.
# POSIX sh, alpine/openssl 3. Do not call from service entrypoints.
set -eu

CERTS="${CERTS:-/certs}"
mkdir -p "$CERTS"

chmod_mesh_files() {
  for f in "$CERTS"/*.crt "$CERTS"/*.key; do
    if [ -f "$f" ]; then
      chmod 644 "$f"
    fi
  done
}

if [ ! -f "$CERTS/ca.crt" ] || [ ! -f "$CERTS/ca.key" ]; then
  openssl genrsa -out "$CERTS/ca.key" 2048
  openssl req -new -x509 -days 3650 \
    -key "$CERTS/ca.key" \
    -out "$CERTS/ca.crt" \
    -subj "/CN=aiforall-platform-mesh-ca"
fi

leaf_ok() {
  crt="$1"
  extra_sans="$2"
  text=$(openssl x509 -in "$crt" -noout -text 2>/dev/null || true)
  printf '%s' "$text" | grep -q "TLS Web Server Authentication" || return 1
  printf '%s' "$text" | grep -q "TLS Web Client Authentication" || return 1
  if [ -n "$extra_sans" ]; then
    dns=${extra_sans#DNS:}
    printf '%s' "$text" | grep -q "DNS:${dns}" || return 1
  fi
  return 0
}

issue_leaf() {
  name="$1"
  extra_sans="${2:-}"
  crt="$CERTS/${name}.crt"
  key="$CERTS/${name}.key"
  if [ -f "$crt" ] && [ -f "$key" ] && leaf_ok "$crt" "$extra_sans"; then
    return 0
  fi
  rm -f "$crt" "$key"
  csr="$CERTS/${name}.csr"
  ext="$CERTS/${name}.ext"
  openssl genrsa -out "$key" 2048
  openssl req -new -key "$key" -out "$csr" -subj "/CN=${name}"
  san="DNS:${name},DNS:localhost,IP:127.0.0.1"
  if [ -n "$extra_sans" ]; then
    san="${san},${extra_sans}"
  fi
  cat > "$ext" <<EOF
basicConstraints=CA:FALSE
subjectAltName=${san}
keyUsage=digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth,clientAuth
EOF
  openssl x509 -req -in "$csr" -CA "$CERTS/ca.crt" -CAkey "$CERTS/ca.key" \
    -CAcreateserial -out "$crt" -days 825 -extfile "$ext"
  rm -f "$csr" "$ext"
}

issue_leaf "iam-service" "DNS:iam.aiforall-platform.svc.cluster.local"
issue_leaf "hive-service"
issue_leaf "telegraph-service"
issue_leaf "manifesto-service"
issue_leaf "github-connect-service"
issue_leaf "gitlab-connect-service"
issue_leaf "envoy-mesh"
issue_leaf "ext-authz" "DNS:ext-authz.aiforall-gateway.svc.cluster.local"
issue_leaf "mesh-client"
chmod_mesh_files
