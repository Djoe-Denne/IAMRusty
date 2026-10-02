#!/bin/sh
# Commande unique :
#   bash scripts/mesh-authn-e2e.sh
#
# Monte le profil mesh (IAM reel, ext-authz Linux, Envoy du depot, mTLS
# platform-mesh, IAM/Hive/Telegraph/Manifesto en mode passerelle), joue tous
# les cas ADR-0308, sort non-zero si un cas rate, puis demonte la pile.
# Relancable apres un arret propre.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

hostpath() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s\n' "$1"
  fi
}

compose() {
  docker compose -f "$ROOT/docker-compose.yml" -f "$ROOT/deploy/mesh/compose.yaml" --profile mesh "$@"
}

RUN="$ROOT/certs/mesh-e2e-run"
FOREIGN="$ROOT/certs/mesh-e2e-foreign"

cleanup() {
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "--- journaux ext-authz / envoy / services ---" >&2
    compose logs --tail 20 create-databases >&2 || true
    compose logs --tail 80 ext-authz envoy-mesh >&2 || true
    compose logs --tail 30 iam-service hive-service telegraph-service manifesto-service >&2 || true
  fi
  echo "--- ext-authz JWKS en clair (derniers refus) ---"
  docker logs --tail 4 mesh-e2e-ext-authz-clearjwks 2>&1 || true
  docker rm -f \
    mesh-e2e-ext-authz-clearjwks \
    mesh-e2e-envoy-clear-authz \
    mesh-e2e-envoy-foreign-authz \
    mesh-e2e-envoy-clear-upstream \
    mesh-e2e-envoy-foreign-upstream \
    mesh-e2e-envoy-clear-jwks \
    >/dev/null 2>&1 || true
  compose down --remove-orphans >/dev/null 2>&1 || true
  exit "$rc"
}
trap cleanup EXIT

mkdir -p "$FOREIGN" "$RUN"

echo "CA etrangere (jetable, hors platform-mesh)"
docker run --rm \
  -v "$(hostpath "$FOREIGN"):/work" \
  alpine:3.20 \
  sh -c 'apk add --no-cache openssl >/dev/null &&
    openssl genrsa -out /work/ca.key 2048 &&
    openssl req -new -x509 -days 2 -key /work/ca.key -out /work/ca.crt -subj /CN=foreign-mesh-ca &&
    openssl genrsa -out /work/client.key 2048 &&
    openssl req -new -key /work/client.key -out /work/client.csr -subj /CN=foreign-client &&
    printf "%s\n" "basicConstraints=CA:FALSE" "subjectAltName=DNS:foreign-client,DNS:localhost,IP:127.0.0.1" "keyUsage=digitalSignature,keyEncipherment" "extendedKeyUsage=serverAuth,clientAuth" > /work/client.ext &&
    openssl x509 -req -in /work/client.csr -CA /work/ca.crt -CAkey /work/ca.key -CAcreateserial -out /work/client.crt -days 2 -extfile /work/client.ext &&
    rm -f /work/client.csr /work/client.ext &&
    chmod 644 /work/*'

tls_cluster() {
  mode="$1"
  sni="$2"
  if [ "$mode" = "clear" ]; then
    return 0
  fi
  cert="/etc/envoy/certs/envoy-mesh.crt"
  key="/etc/envoy/certs/envoy-mesh.key"
  if [ "$mode" = "foreign" ]; then
    cert="/etc/envoy/foreign/client.crt"
    key="/etc/envoy/foreign/client.key"
  fi
  cat <<EOF
      transport_socket:
        name: envoy.transport_sockets.tls
        typed_config:
          "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.UpstreamTlsContext
          sni: ${sni}
          common_tls_context:
            tls_certificates:
              - certificate_chain:
                  filename: ${cert}
                private_key:
                  filename: ${key}
            validation_context:
              trusted_ca:
                filename: /etc/envoy/certs/ca.crt
EOF
}

write_envoy() {
  dest="$1"
  authz_host="$2"
  authz_mode="$3"
  authz_sni="$4"
  upstream_mode="$5"
  upstream_sni="$6"
  scheme=https
  if [ "$authz_mode" = "clear" ]; then
    scheme=http
  fi
  {
    cat <<EOF
static_resources:
  listeners:
    - name: listener_mesh
      address:
        socket_address:
          address: 0.0.0.0
          port_value: 10000
      filter_chains:
        - transport_socket:
            name: envoy.transport_sockets.tls
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext
              require_client_certificate: true
              common_tls_context:
                tls_certificates:
                  - certificate_chain:
                      filename: /etc/envoy/certs/envoy-mesh.crt
                    private_key:
                      filename: /etc/envoy/certs/envoy-mesh.key
                validation_context:
                  trusted_ca:
                    filename: /etc/envoy/certs/ca.crt
          filters:
            - name: envoy.filters.network.http_connection_manager
              typed_config:
                "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
                stat_prefix: mesh_ingress
                route_config:
                  name: local_route
                  virtual_hosts:
                    - name: backend
                      domains: ["*"]
                      routes:
                        - match:
                            prefix: "/iam/"
                          route:
                            cluster: iam
                http_filters:
                  - name: envoy.filters.http.lua
                    typed_config:
                      "@type": type.googleapis.com/envoy.extensions.filters.http.lua.v3.Lua
                      default_source_code:
                        inline_string: |
                          function envoy_on_request(request_handle)
                            local headers = request_handle:headers()
                            local doomed = {}
                            for key, _ in pairs(headers) do
                              if string.sub(string.lower(key), 1, 12) == "x-principal-" then
                                table.insert(doomed, key)
                              end
                            end
                            for _, key in ipairs(doomed) do
                              headers:remove(key)
                            end
                          end
                  - name: envoy.filters.http.ext_authz
                    typed_config:
                      "@type": type.googleapis.com/envoy.extensions.filters.http.ext_authz.v3.ExtAuthz
                      transport_api_version: V3
                      failure_mode_allow: false
                      http_service:
                        server_uri:
                          uri: ${scheme}://${authz_host}:8090/
                          cluster: ext_authz
                          timeout: 5s
                        authorization_request:
                          allowed_headers:
                            patterns:
                              - exact: authorization
                        authorization_response:
                          allowed_upstream_headers:
                            patterns:
                              - exact: x-principal-iss
                              - exact: x-principal-sub
                  - name: envoy.filters.http.router
                    typed_config:
                      "@type": type.googleapis.com/envoy.extensions.filters.http.router.v3.Router
  clusters:
    - name: ext_authz
      connect_timeout: 5s
      type: STRICT_DNS
      lb_policy: ROUND_ROBIN
EOF
    tls_cluster "$authz_mode" "$authz_sni"
    cat <<EOF
      load_assignment:
        cluster_name: ext_authz
        endpoints:
          - lb_endpoints:
              - endpoint:
                  address:
                    socket_address:
                      address: ${authz_host}
                      port_value: 8090
    - name: iam
      connect_timeout: 5s
      type: STRICT_DNS
      lb_policy: ROUND_ROBIN
EOF
    tls_cluster "$upstream_mode" "$upstream_sni"
    upstream_port=8443
    cat <<EOF
      load_assignment:
        cluster_name: iam
        endpoints:
          - lb_endpoints:
              - endpoint:
                  address:
                    socket_address:
                      address: iam-service
                      port_value: ${upstream_port}
admin:
  address:
    socket_address:
      address: 0.0.0.0
      port_value: 9901
EOF
  } > "$dest"
}

echo "Compilation Linux (cache Cargo BuildKit)"
# Service images start FROM local/build-artifacts: build it first, or a
# parallel build copies stale binaries.
compose build build-artifacts
compose build
echo "Pile mesh"
compose up -d

cid=$(compose ps -q iam-service)
net=$(docker inspect -f '{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}' "$cid")
net=${net%% *}

write_envoy "$RUN/clear-authz.yaml" ext-authz clear ext-authz tls iam-service
write_envoy "$RUN/foreign-authz.yaml" ext-authz foreign ext-authz tls iam-service
write_envoy "$RUN/clear-upstream.yaml" ext-authz tls ext-authz clear iam-service
write_envoy "$RUN/foreign-upstream.yaml" ext-authz tls ext-authz foreign iam-service
write_envoy "$RUN/clear-jwks.yaml" mesh-e2e-ext-authz-clearjwks tls ext-authz tls iam-service

start_envoy() {
  name="$1"
  cfg="$2"
  docker rm -f "$name" >/dev/null 2>&1 || true
  MSYS_NO_PATHCONV=1 docker run -d --name "$name" --network "$net" \
    -v "$(hostpath "$cfg"):/etc/envoy/envoy.yaml:ro" \
    -v "$(hostpath "$ROOT/certs/platform-mesh"):/etc/envoy/certs:ro" \
    -v "$(hostpath "$FOREIGN"):/etc/envoy/foreign:ro" \
    envoyproxy/envoy:v1.31-latest \
    -c /etc/envoy/envoy.yaml --log-level warning >/dev/null
}

start_envoy mesh-e2e-envoy-clear-authz "$RUN/clear-authz.yaml"
start_envoy mesh-e2e-envoy-foreign-authz "$RUN/foreign-authz.yaml"
start_envoy mesh-e2e-envoy-clear-upstream "$RUN/clear-upstream.yaml"
start_envoy mesh-e2e-envoy-foreign-upstream "$RUN/foreign-upstream.yaml"
start_envoy mesh-e2e-envoy-clear-jwks "$RUN/clear-jwks.yaml"

docker rm -f mesh-e2e-ext-authz-clearjwks >/dev/null 2>&1 || true
MSYS_NO_PATHCONV=1 docker run -d --name mesh-e2e-ext-authz-clearjwks --network "$net" \
  -e PORT=8090 \
  -e EXT_AUTHZ_AUDIENCE=aiforall \
  -e EXT_AUTHZ_JWKS_URL=http://iam-service:8443/iam/.well-known/jwks.json \
  -e EXT_AUTHZ_TLS_CERT=/app/certs/ext-authz.crt \
  -e EXT_AUTHZ_TLS_KEY=/app/certs/ext-authz.key \
  -e EXT_AUTHZ_TLS_CA=/app/certs/ca.crt \
  -v "$(hostpath "$ROOT/certs/platform-mesh"):/app/certs:ro" \
  aiforall-ext-authz:local >/dev/null

echo "Audience vide"
if docker run --rm \
  -e EXT_AUTHZ_AUDIENCE= \
  -e EXT_AUTHZ_JWKS_URL=https://iam-service:8443/iam/.well-known/jwks.json \
  aiforall-ext-authz:local >/dev/null 2>&1; then
  echo "FAIL audience vide: le process a demarre" >&2
  exit 1
fi
echo "OK audience vide"

echo "Cas"
MSYS_NO_PATHCONV=1 docker run --rm --network "$net" \
  -v "$(hostpath "$ROOT/scripts/mesh-authn-e2e-cases.sh"):/cases.sh:ro" \
  -v "$(hostpath "$ROOT/certs/platform-mesh"):/certs:ro" \
  -v "$(hostpath "$FOREIGN"):/foreign:ro" \
  -v "$(hostpath "$ROOT/IAMRusty/config/keys"):/keys:ro" \
  alpine:3.20 \
  sh /cases.sh
