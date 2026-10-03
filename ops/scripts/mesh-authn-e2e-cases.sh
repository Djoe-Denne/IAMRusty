#!/bin/sh
# Cas isoprod ADR-0308. Lance par ops/scripts/mesh-authn-e2e.sh sur le reseau Compose.
set -eu

apk add --no-cache curl jq openssl python3 postgresql16-client >/dev/null

fail() {
  echo "FAIL $1" >&2
  if [ -f /tmp/body ]; then
    echo "--- body ---" >&2
    cat /tmp/body >&2 || true
    echo >&2
  fi
  exit 1
}

ok() {
  echo "OK $1"
}

b64url_encode() {
  openssl base64 -A | tr '+/' '-_' | tr -d '='
}

b64url_decode() {
  data=$(printf '%s' "$1" | tr '_-' '/+')
  pad=$(( (4 - ${#data} % 4) % 4 ))
  while [ "$pad" -gt 0 ]; do
    data="${data}="
    pad=$((pad - 1))
  done
  printf '%s' "$data" | openssl base64 -d -A
}

CURL="curl -sS --http1.1 --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key"

http_code() {
  # $1 = curl args via env CURL_ARGS already in the command substitution caller
  code=$(curl -sS --http1.1 -o /tmp/body -w '%{http_code}' "$@" ) || code=000
  printf '%s' "$code"
}

# $1 = Envoy host, $2 = cluster, $3 = upstream_rq_* counter.
cluster_stat() {
  curl -fsS "http://$1:9901/stats?filter=cluster.$2.upstream_rq" \
    | awk -v k="cluster.$2.$3:" '$1 == k {print $2}' | tr -d '\r'
}

backend_stat() {
  cluster_stat "$1" iam "$2"
}

rq_total() {
  backend_stat "$1" upstream_rq_total
}

# POSIX sh has no local variables: helper parameters use the req_ prefix so they
# never overwrite the global token, iss and sub.
mesh_curl() {
  req_host="$1"
  req_method="$2"
  req_token="$3"
  shift 3
  # Throwaway Envoys present the envoy-mesh certificate: keep that TLS name.
  if [ -n "$req_token" ]; then
    http_code --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
      --connect-to "envoy-mesh:10000:${req_host}:10000" \
      -X "$req_method" "$@" \
      -H "authorization: Bearer $req_token" \
      "https://envoy-mesh:10000/iam/mesh/echo-headers"
  else
    http_code --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
      --connect-to "envoy-mesh:10000:${req_host}:10000" \
      -X "$req_method" "$@" \
      "https://envoy-mesh:10000/iam/mesh/echo-headers"
  fi
}

authz_stat() {
  curl -fsS "http://$1:9901/stats?filter=ext_authz" \
    | awk -v k="http.mesh_ingress.ext_authz.$2:" '$1 == k {print $2}' | tr -d '\r'
}

# $5 = Envoy ext_authz verdict that must increase: denied (ext-authz refused),
# error (Envoy -> ext-authz hop failed) or ok (ext-authz allowed, upstream refused).
# With ok, Envoy may open a cleartext TCP stream to the TLS upstream, so
# "upstream called" means an upstream 2xx instead of any upstream request.
expect_deny() {
  req_label="$1"
  req_host="$2"
  req_method="$3"
  req_token="$4"
  req_verdict="$5"
  shift 5
  [ -n "$(rq_total "$req_host")" ] || fail "$req_label: admin Envoy injoignable"
  called_stat=upstream_rq_total
  if [ "$req_verdict" = "ok" ]; then
    called_stat=upstream_rq_2xx
  fi
  before=$(backend_stat "$req_host" "$called_stat")
  before=${before:-0}
  verdict_before=$(authz_stat "$req_host" "$req_verdict")
  code=$(mesh_curl "$req_host" "$req_method" "$req_token" "$@")
  after=$(backend_stat "$req_host" "$called_stat")
  after=${after:-0}
  verdict_after=$(authz_stat "$req_host" "$req_verdict")
  case "$code" in
    2*) fail "$req_label: HTTP $code" ;;
  esac
  [ "$before" = "$after" ] || fail "$req_label: upstream appele ($before -> $after) HTTP $code"
  [ "${verdict_after:-0}" -gt "${verdict_before:-0}" ] \
    || fail "$req_label: ext_authz.$req_verdict inchange (HTTP $code)"
  ok "$req_label (HTTP $code, ext_authz.$req_verdict, upstream inchange)"
}

expect_allow() {
  req_label="$1"
  req_host="$2"
  req_method="$3"
  req_token="$4"
  req_iss="$5"
  req_sub="$6"
  shift 6
  [ -n "$req_token" ] || fail "$req_label: token vide"
  before=$(rq_total "$req_host")
  code=$(mesh_curl "$req_host" "$req_method" "$req_token" "$@")
  after=$(rq_total "$req_host")
  if [ "$code" != "200" ]; then
    curl -fsS "http://${req_host}:9901/stats?filter=http.mesh_ingress.ext_authz" >&2 || true
    fail "$req_label: HTTP $code"
  fi
  [ "$after" -gt "$before" ] || fail "$req_label: upstream non appele"
  jq -e --arg iss "$req_iss" --arg sub "$req_sub" --arg method "$req_method" '
    .method == $method
    and .["x-principal"]["x-principal-iss"] == $iss
    and .["x-principal"]["x-principal-sub"] == $sub
    and (.["x-principal"]["x-principal-org"] | not)
    and (.["x-principal"]["x-principal-foo"] | not)
  ' /tmp/body >/dev/null || fail "$req_label: en-tetes upstream"
  ok "$req_label"
}

handshake_fails() {
  label="$1"
  shift
  if curl -sS --http1.1 -o /tmp/body -w '%{http_code}' "$@" >/dev/null 2>/tmp/curlerr; then
    fail "$label: handshake accepte"
  fi
  ok "$label"
}

echo "Attente services et Envoy"
deadline=$(( $(date +%s) + 600 ))
while :; do
  down=""
  for svc in iam hive telegraph manifesto; do
    if ! curl -sS --http1.1 --connect-timeout 3 --max-time 5 \
      --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
      -o /dev/null "https://${svc}-service:8443/${svc}/health" 2>/dev/null; then
      down="$down $svc"
    fi
  done
  envoy_code=$(curl -sS --http1.1 --connect-timeout 3 --max-time 5 \
    --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
    -o /dev/null -w '%{http_code}' "https://envoy-mesh:10000/iam/mesh/echo-headers" 2>/dev/null || true)
  if [ -z "$down" ] && [ "$envoy_code" = "403" ]; then
    break
  fi
  if [ "$(date +%s)" -ge "$deadline" ]; then
    fail "indisponible:${down:- aucun service}, dernier code Envoy ${envoy_code:-none}"
  fi
  sleep 2
done

stamp=$(date +%s)
email="mesh-${stamp}@example.com"
user="mesh${stamp}"
pass="MeshTest1a"

# POST/GET IAM public routes through Envoy without Bearer. ext_authz must not
# increment denied or ok (Check is disabled on these exact paths).
public_envoy() {
  req_label="$1"
  req_method="$2"
  req_path="$3"
  shift 3
  denied_before=$(authz_stat envoy-mesh denied)
  ok_before=$(authz_stat envoy-mesh ok)
  code=$(http_code --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
    -X "$req_method" "$@" "https://envoy-mesh:10000${req_path}")
  denied_after=$(authz_stat envoy-mesh denied)
  ok_after=$(authz_stat envoy-mesh ok)
  case "$code" in
    2*) ;;
    *) fail "$req_label: HTTP $code" ;;
  esac
  [ "${denied_after:-0}" = "${denied_before:-0}" ] \
    || fail "$req_label: ext_authz.denied (${denied_before:-0} -> ${denied_after:-0})"
  [ "${ok_after:-0}" = "${ok_before:-0}" ] \
    || fail "$req_label: ext_authz.ok (${ok_before:-0} -> ${ok_after:-0})"
  ok "$req_label (HTTP $code, ext_authz inchange)"
}

echo "Emission JWT IAM"
public_envoy "signup via Envoy sans Bearer" POST /iam/api/auth/signup \
  -H 'content-type: application/json' \
  -d "{\"email\":\"${email}\",\"password\":\"${pass}\"}"
reg=$(jq -r '.registration_token // empty' /tmp/body)
[ -n "$reg" ] || fail "signup sans registration_token"

curl -sS --http1.1 --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
  -H 'content-type: application/json' \
  -d "{\"registration_token\":\"${reg}\",\"username\":\"${user}\"}" \
  -o /tmp/body \
  "https://iam-service:8443/iam/api/auth/complete-registration" || fail "complete-registration"

updated=$(PGPASSWORD=postgres psql -h postgres -U postgres -d iam_dev -v ON_ERROR_STOP=1 -tAc \
  "UPDATE user_emails SET is_verified = true WHERE email = '${email}' RETURNING email;")
printf '%s' "$updated" | grep -q "$email" || fail "email non verifie en base: ${updated}"

public_envoy "login via Envoy sans Bearer" POST /iam/api/auth/login \
  -H 'content-type: application/json' \
  -d "{\"email\":\"${email}\",\"password\":\"${pass}\"}"
token=$(jq -r '.access_token // empty' /tmp/body)
[ -n "$token" ] || fail "login sans access_token"

header_b64=$(printf '%s' "$token" | cut -d. -f1)
payload_b64=$(printf '%s' "$token" | cut -d. -f2)
payload_json=$(b64url_decode "$payload_b64")
iss=$(printf '%s' "$payload_json" | jq -r '.iss')
sub=$(printf '%s' "$payload_json" | jq -r '.sub')
kid=$(b64url_decode "$header_b64" | jq -r '.kid')
[ -n "$iss" ] && [ "$iss" != "null" ] || fail "iss absent"
[ -n "$sub" ] && [ "$sub" != "null" ] || fail "sub absent"

public_envoy "JWKS via Envoy sans Bearer" GET /iam/.well-known/jwks.json
n=$(jq -r --arg kid "$kid" '.keys[] | select(.kid==$kid) | .n' /tmp/body)
[ -n "$n" ] && [ "$n" != "null" ] || fail "kid $kid absent du JWKS IAM"
mod=$(openssl rsa -pubin -in /keys/test-platform.pub -modulus -noout | cut -d= -f2)
n_pem=$(python3 -c 'import base64,sys; print(base64.urlsafe_b64encode(bytes.fromhex(sys.argv[1])).decode().rstrip("="))' "$mod")
[ "$n" = "$n_pem" ] || fail "la cle PEM IAM ne correspond pas au JWKS"

sign_token() {
  payload_json="$1"
  payload_b64=$(printf '%s' "$payload_json" | b64url_encode)
  signing="${header_b64}.${payload_b64}"
  sig=$(printf '%s' "$signing" | openssl dgst -sha256 -sign /keys/test-platform.pem | b64url_encode)
  printf '%s' "${signing}.${sig}"
}

bad_aud=$(sign_token "$(printf '%s' "$payload_json" | jq -c '.aud="not-aiforall"')")
expired=$(sign_token "$(printf '%s' "$payload_json" | jq -c '.exp=1')")
sigpart=$(printf '%s' "$token" | awk -F. '{print $3}')
last=$(printf '%s' "$sigpart" | awk '{print substr($0, length($0), 1)}')
if [ "$last" = "A" ]; then
  flip=B
else
  flip=A
fi
bad_sig=$(printf '%s' "$token" | awk -v flip="$flip" -F. '{print $1"."$2"."substr($3,1,length($3)-1) flip}')
unknown_header=$(b64url_decode "$header_b64" | jq -c '.kid="unknown-kid-not-in-jwks"' | b64url_encode)
unknown="${unknown_header}.${payload_b64}.$(printf '%s' "$token" | awk -F. '{print $3}')"

expect_deny "sans JWT avec spoofs" envoy-mesh GET "" denied \
  -H "x-principal-iss: spoof-iss" \
  -H "x-principal-sub: spoof-sub" \
  -H "x-principal-org: spoof-org" \
  -H "x-principal-foo: spoof-foo"

expect_allow "JWT valide plus spoofs" envoy-mesh GET "$token" "$iss" "$sub" \
  -H "x-principal-iss: spoof-iss" \
  -H "x-principal-sub: spoof-sub" \
  -H "x-principal-org: spoof-org" \
  -H "x-principal-foo: spoof-foo"

expect_allow "JWT valide sans spoof" envoy-mesh GET "$token" "$iss" "$sub"

expect_deny "mauvaise signature" envoy-mesh GET "$bad_sig" denied
expect_deny "aud incorrect" envoy-mesh GET "$bad_aud" denied
expect_deny "expire" envoy-mesh GET "$expired" denied
expect_deny "kid inconnu" envoy-mesh GET "$unknown" denied

expect_deny "Envoy vers ext-authz en clair" mesh-e2e-envoy-clear-authz GET "$token" error
expect_deny "Envoy vers ext-authz CA etrangere" mesh-e2e-envoy-foreign-authz GET "$token" error
expect_deny "Envoy vers upstream en clair" mesh-e2e-envoy-clear-upstream GET "$token" ok
expect_deny "Envoy vers upstream CA etrangere" mesh-e2e-envoy-foreign-upstream GET "$token" ok
expect_deny "ext-authz vers JWKS en clair" mesh-e2e-envoy-clear-jwks GET "$token" denied

expect_allow "mTLS valide et JWT IAM" envoy-mesh POST "$token" "$iss" "$sub"

handshake_fails "client vers Envoy sans certificat" \
  --cacert /certs/ca.crt "https://envoy-mesh:10000/iam/mesh/echo-headers"
handshake_fails "client vers Envoy CA etrangere" \
  --cacert /certs/ca.crt --cert /foreign/client.crt --key /foreign/client.key \
  "https://envoy-mesh:10000/iam/mesh/echo-headers"
handshake_fails "Envoy vers ext-authz deja couvert; probe ext-authz sans certificat" \
  --cacert /certs/ca.crt "https://ext-authz:8090/"
handshake_fails "probe ext-authz CA etrangere" \
  --cacert /certs/ca.crt --cert /foreign/client.crt --key /foreign/client.key \
  "https://ext-authz:8090/"
handshake_fails "IAM :8443 sans certificat" \
  --cacert /certs/ca.crt "https://iam-service:8443/iam/health"
handshake_fails "IAM :8443 CA etrangere" \
  --cacert /certs/ca.crt --cert /foreign/client.crt --key /foreign/client.key \
  "https://iam-service:8443/iam/health"
handshake_fails "IAM :8443 en clair" \
  "http://iam-service:8443/iam/health"
handshake_fails "ext-authz en clair" \
  "http://ext-authz:8090/"

echo "Services en mode passerelle"
# Services take the principal only from envoy-mesh (certificate SAN) over mTLS
# and never read the bearer JWT. Every path around Envoy must answer 401.

gateway_call() {
  req_method="$1"
  req_path="$2"
  req_token="$3"
  shift 3
  if [ -n "$req_token" ]; then
    set -- "$@" -H "authorization: Bearer $req_token"
  fi
  http_code --cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key \
    -X "$req_method" "$@" "https://envoy-mesh:10000${req_path}"
}

# label cluster method path: no JWT, spoofed principal of the real user.
service_deny() {
  req_label="$1"
  req_cluster="$2"
  before=$(cluster_stat envoy-mesh "$req_cluster" upstream_rq_total)
  denied_before=$(authz_stat envoy-mesh denied)
  code=$(gateway_call "$3" "$4" "" -H "x-principal-iss: $iss" -H "x-principal-sub: $sub")
  after=$(cluster_stat envoy-mesh "$req_cluster" upstream_rq_total)
  denied_after=$(authz_stat envoy-mesh denied)
  [ "$code" = "403" ] || fail "$req_label: HTTP $code"
  [ -n "$before" ] && [ "$before" = "$after" ] \
    || fail "$req_label: $req_cluster appele (${before:-?} -> ${after:-?})"
  [ "${denied_after:-0}" -gt "${denied_before:-0}" ] || fail "$req_label: ext_authz.denied inchange"
  ok "$req_label (HTTP 403, $req_cluster non appele)"
}

# label cluster method path jq-filter [curl args]: the filter sees $sub, $iss, $email.
service_allow() {
  req_label="$1"
  req_cluster="$2"
  req_method="$3"
  req_path="$4"
  req_filter="$5"
  shift 5
  before=$(cluster_stat envoy-mesh "$req_cluster" upstream_rq_2xx)
  code=$(gateway_call "$req_method" "$req_path" "$token" "$@")
  after=$(cluster_stat envoy-mesh "$req_cluster" upstream_rq_2xx)
  case "$code" in
    2*) ;;
    *) fail "$req_label: HTTP $code" ;;
  esac
  [ "${after:-0}" -gt "${before:-0}" ] || fail "$req_label: $req_cluster non appele"
  jq -e --arg sub "$sub" --arg iss "$iss" --arg email "$email" "$req_filter" /tmp/body >/dev/null \
    || fail "$req_label: principal non utilise par le service"
  ok "$req_label (HTTP $code)"
}

# label url [curl args]: straight to the service, around Envoy.
direct_unauthorized() {
  req_label="$1"
  req_url="$2"
  shift 2
  code=$(http_code "$@" "$req_url")
  [ "$code" = "401" ] || fail "$req_label: HTTP $code"
  ok "$req_label (HTTP 401)"
}

MESH_CLIENT="--cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key"
SPOOF_ISS="x-principal-iss: $iss"
SPOOF_SUB="x-principal-sub: $sub"
BEARER="authorization: Bearer $token"

service_allow "IAM /api/me via Envoy" iam GET /iam/api/me \
  '.id == $sub and .email == $email'
service_allow "Hive cree une organisation via Envoy" hive POST /hive/api/organizations \
  '.owner_user_id == $sub' \
  -H 'content-type: application/json' \
  -d "{\"name\":\"Mesh org ${stamp}\",\"slug\":\"mesh-org-${stamp}\"}"
org_id=$(jq -r '.id // empty' /tmp/body)
[ -n "$org_id" ] && [ "$org_id" != "null" ] || fail "organisation sans id"
[ -n "${OPENFGA_STORE_ID:-}" ] || fail "OPENFGA_STORE_ID absent (bootstrap e2e)"
write_body=$(jq -n --arg user "user:${sub}" --arg object "organization:${org_id}" \
  '{writes:{tuple_keys:[{user:$user,relation:"owner",object:$object}]}}')
write_code=$(http_code -H 'content-type: application/json' -d "$write_body" \
  -X POST "http://openfga:8080/stores/${OPENFGA_STORE_ID}/write")
[ "$write_code" = "200" ] || fail "OpenFGA write owner: HTTP $write_code"
ok "OpenFGA owner tuple pour l organisation e2e"
service_allow "Hive membre owner porte l'issuer du JWT" hive GET \
  "/hive/api/organizations/${org_id}/members/${sub}" \
  '.issuer == $iss and .user_id == $sub'
service_allow "Telegraph notifications portent user_id du JWT" telegraph GET /telegraph/api/notifications \
  '.user_id == $sub'
service_allow "Manifesto cree un projet via Envoy" manifesto POST /manifesto/api/projects \
  '.owner_id == $sub and .created_by == $sub' \
  -H 'content-type: application/json' \
  -d "{\"name\":\"Mesh project ${stamp}\",\"owner_type\":\"personal\"}"

for route in "iam GET /iam/api/me" "hive GET /hive/api/organizations" \
  "telegraph GET /telegraph/api/notifications" "manifesto POST /manifesto/api/projects"; do
  set -- $route
  service_deny "$1 sans JWT via Envoy, principal usurpe" "$1" "$2" "$3"
  svc="$1"
  method="$2"
  path="$3"
  # shellcheck disable=SC2086
  direct_unauthorized "$svc :8443 JWT valide sans Envoy" "https://${svc}-service:8443${path}" \
    $MESH_CLIENT -X "$method" -H "$BEARER"
  # shellcheck disable=SC2086
  direct_unauthorized "$svc :8443 principal usurpe (cert mesh-client)" "https://${svc}-service:8443${path}" \
    $MESH_CLIENT -X "$method" -H "$SPOOF_ISS" -H "$SPOOF_SUB"
  direct_unauthorized "$svc :8080 HTTP principal usurpe et JWT" "http://${svc}-service:8080${path}" \
    -X "$method" -H "$SPOOF_ISS" -H "$SPOOF_SUB" -H "$BEARER"
  handshake_fails "$svc :8443 sans certificat client" \
    --cacert /certs/ca.crt "https://${svc}-service:8443/${svc}/health"
done

# Optional routes: mesh-client / HTTP must not continue anonymous.
direct_unauthorized "Hive search :8443 mesh-client + spoof" \
  "https://hive-service:8443/hive/api/organizations/search?query=mesh" \
  $MESH_CLIENT -H "$SPOOF_ISS" -H "$SPOOF_SUB"
direct_unauthorized "Hive search :8080 HTTP + spoof" \
  "http://hive-service:8080/hive/api/organizations/search?query=mesh" \
  -H "$SPOOF_ISS" -H "$SPOOF_SUB"
direct_unauthorized "Manifesto projects :8443 mesh-client + spoof" \
  "https://manifesto-service:8443/manifesto/api/projects" \
  $MESH_CLIENT -H "$SPOOF_ISS" -H "$SPOOF_SUB"
direct_unauthorized "Manifesto projects :8080 HTTP + spoof" \
  "http://manifesto-service:8080/manifesto/api/projects" \
  -H "$SPOOF_ISS" -H "$SPOOF_SUB"

search_before=$(cluster_stat envoy-mesh hive upstream_rq_total)
search_code=$(gateway_call GET "/hive/api/organizations/search?query=mesh" "$token")
search_after=$(cluster_stat envoy-mesh hive upstream_rq_total)
[ "$search_code" != "401" ] || fail "Hive search via Envoy + JWT: HTTP $search_code"
[ "${search_after:-0}" -gt "${search_before:-0}" ] || fail "Hive search via Envoy: hive non appele"
ok "Hive search via Envoy + JWT (HTTP $search_code, hive appele)"

anon_code=$(http_code --cacert /certs/ca.crt --cert /certs/envoy-mesh.crt --key /certs/envoy-mesh.key \
  "https://hive-service:8443/hive/api/organizations/search?query=mesh")
[ "$anon_code" = "200" ] || fail "Hive search envoy-mesh sans principal: HTTP $anon_code"
ok "Hive search envoy-mesh sans principal (HTTP 200 anonyme)"

# Hive signer via Envoy (user JWT) then IAM S2S on :8080 with internal token.
pubkey=$(cat /keys/test-platform.pub)
signer_body=$(jq -n --arg ref "${org_id}/e2e.pem" --arg pk "$pubkey" \
  '{provider_type:"pem_file",provider_key_ref:$ref,public_key:$pk}')
signer_path="/hive/api/organizations/${org_id}/signer/configure"
signer_before=$(cluster_stat envoy-mesh hive upstream_rq_total)
signer_code=$(gateway_call POST "$signer_path" "$token" \
  -H 'content-type: application/json' -d "$signer_body")
signer_after=$(cluster_stat envoy-mesh hive upstream_rq_total)
[ "$signer_code" != "401" ] || fail "Hive signer configure via Envoy: HTTP 401"
[ "${signer_after:-0}" -gt "${signer_before:-0}" ] || fail "Hive signer configure: hive non appele"
case "$signer_code" in
  2*)
    jq -e '.kid and .signing_profile_id' /tmp/body >/dev/null \
      || fail "Hive signer configure via Envoy: corps incomplet"
    ok "Hive signer configure via Envoy (HTTP $signer_code)"
    ;;
  *)
    fail "Hive signer configure via Envoy: HTTP $signer_code"
    ;;
esac
direct_unauthorized "Hive signer bypass Envoy mesh-client + Bearer" \
  "https://hive-service:8443${signer_path}" $MESH_CLIENT -X POST -H "$BEARER" \
  -H 'content-type: application/json' -d "$signer_body"
direct_unauthorized "Hive signer bypass Envoy mesh-client + spoof" \
  "https://hive-service:8443${signer_path}" $MESH_CLIENT -X POST \
  -H "$SPOOF_ISS" -H "$SPOOF_SUB" \
  -H 'content-type: application/json' -d "$signer_body"
iam_code=$(http_code -X POST \
  "http://iam-service:8080/iam/internal/organizations/${org_id}/signer/test")
case "$iam_code" in
  401|403) ok "IAM signer interne sans token (HTTP $iam_code)" ;;
  *) fail "IAM signer interne sans token: HTTP $iam_code" ;;
esac

code=$(http_code --cacert /certs/ca.crt --cert /certs/envoy-mesh.crt --key /certs/envoy-mesh.key \
  -H "$SPOOF_ISS" -H "$SPOOF_SUB" "https://iam-service:8443/iam/api/me")
[ "$code" = "200" ] && jq -e --arg sub "$sub" '.id == $sub' /tmp/body >/dev/null \
  || fail "temoin cle envoy-mesh: HTTP $code"
ok "temoin : seule la cle envoy-mesh injecte un principal (HTTP 200)"

ok "tous les cas"
