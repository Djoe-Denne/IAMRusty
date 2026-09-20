#!/bin/sh
# Generate-if-absent platform mesh CA + per-service leaves for Hive/IAM/Telegraph.
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

issue_leaf() {
  name="$1"
  crt="$CERTS/${name}.crt"
  key="$CERTS/${name}.key"
  if [ -f "$crt" ] && [ -f "$key" ]; then
    return 0
  fi
  csr="$CERTS/${name}.csr"
  ext="$CERTS/${name}.ext"
  openssl genrsa -out "$key" 2048
  openssl req -new -key "$key" -out "$csr" -subj "/CN=${name}"
  cat > "$ext" <<EOF
subjectAltName=DNS:${name},DNS:localhost,IP:127.0.0.1
EOF
  openssl x509 -req -in "$csr" -CA "$CERTS/ca.crt" -CAkey "$CERTS/ca.key" \
    -CAcreateserial -out "$crt" -days 825 -extfile "$ext"
  rm -f "$csr" "$ext"
}

issue_leaf "iam-service"
issue_leaf "hive-service"
issue_leaf "telegraph-service"
chmod_mesh_files
