#!/bin/sh
# Parent execution only. Create fresh, separate local transport/workload CAs.
# Refuse any pre-existing content; no key/leaf overwrite or automatic rotation.
set -eu
: "${LOCAL_FULL_CERT_DIR:?set an explicit new directory}"
if [ -e "$LOCAL_FULL_CERT_DIR" ]; then
  echo 'certificate output path already exists; no overwrite' >&2
  exit 1
fi
umask 077
mkdir -p "$LOCAL_FULL_CERT_DIR/mesh" "$LOCAL_FULL_CERT_DIR/workload"
ca() {
  dir="$1"; cn="$2"
  openssl req -x509 -newkey rsa:2048 -nodes -days 365 \
    -keyout "$dir/ca.key" -out "$dir/ca.crt" -subj "/CN=$cn" \
    -addext 'basicConstraints=critical,CA:TRUE' -addext 'keyUsage=critical,keyCertSign,cRLSign' >/dev/null 2>&1
}
leaf() {
  dir="$1"; name="$2"; sans="$3"
  openssl req -new -newkey rsa:2048 -nodes -keyout "$dir/$name.key" \
    -out "$dir/$name.csr" -subj "/CN=$name" >/dev/null 2>&1
  printf 'basicConstraints=critical,CA:FALSE\nsubjectAltName=%s\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth,clientAuth\n' "$sans" > "$dir/$name.ext"
  openssl x509 -req -in "$dir/$name.csr" -CA "$dir/ca.crt" -CAkey "$dir/ca.key" \
    -CAcreateserial -days 90 -out "$dir/$name.crt" -extfile "$dir/$name.ext" >/dev/null 2>&1
}
ca "$LOCAL_FULL_CERT_DIR/mesh" aiforall-local-full-transport
ca "$LOCAL_FULL_CERT_DIR/workload" aiforall-local-full-workload
for name in iam hive telegraph manifesto; do
  leaf "$LOCAL_FULL_CERT_DIR/mesh" "$name-service" "DNS:$name-service,DNS:$name,DNS:$name.aiforall-local-full.svc.cluster.local,DNS:localhost,IP:127.0.0.1"
done
for name in envoy-mesh ext-authz mesh-client; do
  leaf "$LOCAL_FULL_CERT_DIR/mesh" "$name" "DNS:$name,DNS:$name.aiforall-local-full.svc.cluster.local,DNS:localhost,IP:127.0.0.1"
done
leaf "$LOCAL_FULL_CERT_DIR/workload" server 'DNS:lazaret-service,DNS:lazaret,DNS:lazaret.aiforall-local-full.svc.cluster.local,DNS:localhost,IP:127.0.0.1'
echo 'created separate local-full CA/leaf material; contents not printed; parent owns directory'
