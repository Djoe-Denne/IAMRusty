#!/bin/sh
# Kind mesh contract: deny without JWT, allow with the gateway principal used,
# mesh-client bypass refused, including S2S token/revoke without a user JWT.
# Public signup/login/JWKS go through Envoy without Check.
set -eu

fail() {
  echo "FAIL $*" >&2
  # Responses can contain registration, user, or provider tokens: never dump them.
  exit 1
}
ok() { echo "OK $*"; }

apk add --no-cache curl jq openssl python3 postgresql16-client >/dev/null

b64url_decode() {
  data=$(printf '%s' "$1" | tr '_-' '/+')
  pad=$(( (4 - ${#data} % 4) % 4 ))
  while [ "$pad" -gt 0 ]; do
    data="${data}="
    pad=$((pad - 1))
  done
  printf '%s' "$data" | openssl base64 -d -A
}

http_code() {
  code=$(curl -sS --http1.1 -o /tmp/body -w '%{http_code}' "$@") || code=000
  printf '%s' "$code"
}

CERTS="--cacert /certs/ca.crt --cert /certs/mesh-client.crt --key /certs/mesh-client.key"
# Short name matches the envoy-mesh certificate SAN. The cluster FQDN does not.
ENVOY="https://envoy-mesh:10000"
IAM="https://iam.aiforall-platform.svc.cluster.local:8443"
stamp=$(date +%s)
email="kindmesh-${stamp}@example.com"
user="kindmesh${stamp}"
pass="MeshTest1a"
SPOOF_ISS="x-principal-iss: http://127.0.0.1:8080/iam"
SPOOF_SUB="x-principal-sub: 00000000-0000-0000-0000-000000000099"

public_envoy() {
  label=$1
  method=$2
  path=$3
  shift 3
  code=$(http_code $CERTS \
    -X "$method" "$@" "${ENVOY}${path}")
  case "$code" in
    2*) ok "$label (HTTP $code)" ;;
    *) fail "$label: HTTP $code" ;;
  esac
}

echo "Emission JWT IAM"
public_envoy "signup via Envoy sans Bearer" POST /iam/api/auth/signup \
  -H 'content-type: application/json' \
  -d "{\"email\":\"${email}\",\"password\":\"${pass}\"}"
reg=$(jq -r '.registration_token // empty' /tmp/body)
[ -n "$reg" ] || fail "signup sans registration_token"

curl -sS --http1.1 $CERTS \
  -H 'content-type: application/json' \
  -d "{\"registration_token\":\"${reg}\",\"username\":\"${user}\"}" \
  -o /tmp/body \
  "${IAM}/iam/api/auth/complete-registration" || fail "complete-registration"

updated=$(PGPASSWORD=postgres psql -h postgres.aiforall-platform.svc.cluster.local -U postgres -d iam_dev -v ON_ERROR_STOP=1 -tAc \
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
[ -n "$n" ] && [ "$n" != "null" ] || fail "kid $kid absent du JWKS"

code=$(http_code $CERTS \
  "${ENVOY}/iam/api/me")
[ "$code" = "401" ] || [ "$code" = "403" ] || fail "IAM /me sans JWT: HTTP $code"
ok "IAM /me sans JWT refuse (HTTP $code)"

code=$(http_code $CERTS \
  -H "authorization: Bearer ${token}" "${ENVOY}/iam/api/me")
[ "$code" = "200" ] || fail "IAM /me via Envoy: HTTP $code"
jq -e --arg sub "$sub" '.id == $sub' /tmp/body >/dev/null || fail "IAM /me n a pas utilise sub"
ok "IAM /me via Envoy utilise sub"

code=$(http_code $CERTS \
  -H "authorization: Bearer ${token}" "${ENVOY}/telegraph/api/notifications")
[ "$code" = "200" ] || fail "Telegraph via Envoy: HTTP $code"
jq -e --arg sub "$sub" '.user_id == $sub' /tmp/body >/dev/null || fail "Telegraph user_id != sub"
ok "Telegraph notifications portent user_id du JWT"

code=$(http_code $CERTS \
  -H "$SPOOF_ISS" -H "$SPOOF_SUB" \
  "https://telegraph.aiforall-platform.svc.cluster.local:8443/telegraph/api/notifications")
[ "$code" = "401" ] || fail "Telegraph bypass mesh-client: HTTP $code"
ok "Telegraph bypass mesh-client refuse (HTTP 401)"

code=$(http_code $CERTS \
  -H "$SPOOF_ISS" -H "$SPOOF_SUB" \
  "https://hive.aiforall-platform.svc.cluster.local:8443/hive/api/organizations/search?query=mesh")
[ "$code" = "401" ] || fail "Hive search bypass mesh-client: HTTP $code"
ok "Hive search bypass mesh-client refuse (HTTP 401)"

# Exercise the gateway -> IAM S2S hop, not a downstream user request to Envoy.
# Only the gateway certificate can supply this principal (ADR-0308 section 7).
# No user JWT is sent on the authorized hop; the existing target-user contract stays.
GATEWAY_CERTS="--cacert /certs/ca.crt --cert /certs/envoy-mesh.crt --key /certs/envoy-mesh.key"
INTERNAL_HEADER="x-iam-internal-token: iam-internal-dev-token"
PRINCIPAL_ISS="x-principal-iss: ${iss}"
PRINCIPAL_SUB="x-principal-sub: ${sub}"
provider_token="kind-mesh-provider-fixture-${stamp}"
PGPASSWORD=postgres psql -h postgres.aiforall-platform.svc.cluster.local -U postgres -d iam_dev \
  -v ON_ERROR_STOP=1 -q -c \
  "INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, expires_in)
   VALUES ('${sub}', 'github', 'kind-mesh-${stamp}', '${provider_token}', 3600);" >/dev/null

for operation in token revoke; do
  case "$operation" in token) method=POST ;; revoke) method=DELETE ;; esac
  # The certificate authenticates the hop, not a target user. In mesh mode a
  # Bearer cannot replace the gateway principal, even with the correct gate.
  code=$(http_code $GATEWAY_CERTS -X "$method" -H "$INTERNAL_HEADER" \
    -H "authorization: Bearer ${token}" "${IAM}/iam/internal/github/${operation}")
  [ "$code" = "401" ] || fail "IAM S2S ${operation} envoy-mesh sans principal: HTTP $code"
  ok "IAM S2S ${operation} envoy-mesh + gate + Bearer sans principal refuse (HTTP 401)"
  for credentials in bearer spoof internal all; do
    case "$credentials" in
      bearer) set -- -H "authorization: Bearer ${token}" ;;
      spoof) set -- -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" ;;
      internal) set -- -H "$INTERNAL_HEADER" ;;
      all) set -- -H "authorization: Bearer ${token}" -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" -H "$INTERNAL_HEADER" ;;
    esac
    code=$(http_code $CERTS -X "$method" "$@" "${IAM}/iam/internal/github/${operation}")
    [ "$code" = "401" ] || fail "IAM S2S ${operation} mesh-client ${credentials}: HTTP $code"
    ! grep -q '"access_token"' /tmp/body \
      || fail "IAM S2S ${operation} mesh-client ${credentials} divulgue access_token"
    ok "IAM S2S ${operation} mesh-client ${credentials} refuse (HTTP 401)"
  done
  # Privileged fixture already mounts these certificates. Do not read/export
  # their contents: exercise each real application identity directly with curl.
  for app in iam hive telegraph manifesto; do
    APP_CERTS="--cacert /certs/ca.crt --cert /certs/${app}-service.crt --key /certs/${app}-service.key"
    code=$(http_code $APP_CERTS -X "$method" -H "authorization: Bearer ${token}" \
      -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" -H "$INTERNAL_HEADER" \
      "${IAM}/iam/internal/github/${operation}")
    [ "$code" = "401" ] || fail "IAM S2S ${operation} ${app}-service direct: HTTP $code"
    ! grep -q '"access_token"' /tmp/body \
      || fail "IAM S2S ${operation} ${app}-service divulgue access_token"
    ok "IAM S2S ${operation} ${app}-service + Bearer + principal + gate refuse (HTTP 401)"
  done
  for gate in missing wrong; do
    case "$gate" in
      missing) set -- ;;
      wrong) set -- -H 'x-iam-internal-token: wrong-internal-token' ;;
    esac
    code=$(http_code $GATEWAY_CERTS -X "$method" -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" \
      "$@" "${IAM}/iam/internal/github/${operation}")
    [ "$code" = "403" ] || fail "IAM S2S ${operation} gate ${gate}: HTTP $code"
    jq -e 'has("access_token") | not' /tmp/body >/dev/null \
      || fail "IAM S2S ${operation} gate ${gate} divulgue access_token"
    ok "IAM S2S ${operation} envoy-mesh gate ${gate} refuse (HTTP 403)"
  done
done

# Denied requests must leave the real provider fixture intact, notably DELETE.
remaining=$(PGPASSWORD=postgres psql -h postgres.aiforall-platform.svc.cluster.local -U postgres -d iam_dev \
  -v ON_ERROR_STOP=1 -tAc "SELECT count(*) FROM provider_tokens WHERE user_id = '${sub}' AND provider = 'github';")
[ "$remaining" = "1" ] || fail "IAM S2S refus a modifie le token provider"

code=$(http_code $GATEWAY_CERTS -X POST -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" \
  -H "$INTERNAL_HEADER" "${IAM}/iam/internal/github/token")
[ "$code" = "200" ] || fail "IAM S2S token envoy-mesh sans JWT: HTTP $code"
jq -e --arg expected "$provider_token" '.access_token == $expected' /tmp/body >/dev/null \
  || fail "IAM S2S token n a pas utilise le principal passerelle"
ok "IAM S2S token envoy-mesh + gate sans JWT (HTTP 200)"

code=$(http_code $GATEWAY_CERTS -X DELETE -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" \
  -H "$INTERNAL_HEADER" "${IAM}/iam/internal/github/revoke")
[ "$code" = "200" ] || fail "IAM S2S revoke envoy-mesh sans JWT: HTTP $code"
remaining=$(PGPASSWORD=postgres psql -h postgres.aiforall-platform.svc.cluster.local -U postgres -d iam_dev \
  -v ON_ERROR_STOP=1 -tAc "SELECT count(*) FROM provider_tokens WHERE user_id = '${sub}' AND provider = 'github';")
[ "$remaining" = "0" ] || fail "IAM S2S revoke n a pas supprime le token du principal passerelle"
ok "IAM S2S revoke envoy-mesh + gate sans JWT supprime le token (HTTP 200)"

code=$(http_code $GATEWAY_CERTS -X POST -H "$PRINCIPAL_ISS" -H "$PRINCIPAL_SUB" \
  -H "$INTERNAL_HEADER" "${IAM}/iam/internal/github/token")
[ "$code" = "404" ] || fail "IAM S2S token apres revoke: HTTP $code"
jq -e 'has("access_token") | not' /tmp/body >/dev/null \
  || fail "IAM S2S token apres revoke divulgue access_token"
ok "IAM S2S token apres revoke introuvable sans JWT (HTTP 404)"

if [ -n "${OPENFGA_STORE_ID:-}" ]; then
  code=$(http_code $CERTS \
    -H "authorization: Bearer ${token}" -H 'content-type: application/json' \
    -d "{\"name\":\"Kind org ${stamp}\",\"slug\":\"kind-org-${stamp}\"}" \
    -X POST "${ENVOY}/hive/api/organizations")
  [ "$code" = "200" ] || [ "$code" = "201" ] || fail "Hive cree une organisation: HTTP $code"
  org_id=$(jq -r '.id // empty' /tmp/body)
  [ -n "$org_id" ] || fail "organisation sans id"
  write_body=$(jq -n --arg user "user:${sub}" --arg object "organization:${org_id}" \
    '{writes:{tuple_keys:[{user:$user,relation:"owner",object:$object}]}}')
  wcode=$(http_code -H 'content-type: application/json' -d "$write_body" \
    -X POST "http://openfga.aiforall-platform.svc.cluster.local:8080/stores/${OPENFGA_STORE_ID}/write")
  [ "$wcode" = "200" ] || fail "OpenFGA write owner: HTTP $wcode"
  code=$(http_code $CERTS \
    -H "authorization: Bearer ${token}" \
    "${ENVOY}/hive/api/organizations/${org_id}/members/${sub}")
  [ "$code" = "200" ] || fail "Hive membre: HTTP $code"
  jq -e --arg sub "$sub" --arg iss "$iss" '.issuer == $iss and .user_id == $sub' /tmp/body >/dev/null \
    || fail "Hive membre issuer/user_id"
  ok "Hive membre owner porte l issuer du JWT"
fi

echo "KIND MESH E2E OK"
