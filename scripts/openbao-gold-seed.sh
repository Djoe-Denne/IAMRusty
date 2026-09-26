#!/bin/sh
# Seed OpenBao compose : KV plugin (Lazaret) + Transit Cosign (Adm-A) + policies.
# Root UNIQUEMENT ici et dans le service compose openbao-seed.
# Auth Kubernetes : si KUBERNETES_HOST + CA + TOKEN_REVIEWER_JWT (prove-gold), pas AppRole.
set -eu
BAO_ADDR="${BAO_ADDR:-http://openbao:8200}"
BAO_TOKEN="${BAO_TOKEN:-lazaret-dev-root}"
export BAO_ADDR VAULT_ADDR="$BAO_ADDR" BAO_TOKEN VAULT_TOKEN="$BAO_TOKEN"

bao kv put secret/ops/token token=compose-dev-secret

# Transit mount (ignore already enabled)
bao secrets enable transit 2>/dev/null || true
# ECDSA P-256 key for Cosign hashivault://apparatus-p4-cosign
bao write -f transit/keys/apparatus-p4-cosign type=ecdsa-p256 2>/dev/null || true

bao policy write apparatus-transit-sign - <<'EOF'
path "transit/sign/apparatus-p4-cosign" {
  capabilities = ["update"]
}
path "transit/sign/apparatus-p4-cosign/*" {
  capabilities = ["update"]
}
path "transit/hmac/apparatus-p4-cosign" {
  capabilities = ["update"]
}
path "transit/hmac/apparatus-p4-cosign/*" {
  capabilities = ["update"]
}
path "transit/verify/apparatus-p4-cosign" {
  capabilities = ["update"]
}
path "transit/verify/apparatus-p4-cosign/*" {
  capabilities = ["update"]
}
path "transit/keys/apparatus-p4-cosign" {
  capabilities = ["read"]
}
EOF

bao policy write apparatus-transit-verify - <<'EOF'
path "transit/verify/apparatus-p4-cosign" {
  capabilities = ["update"]
}
path "transit/verify/apparatus-p4-cosign/*" {
  capabilities = ["update"]
}
path "transit/keys/apparatus-p4-cosign" {
  capabilities = ["read"]
}
EOF

bao policy write apparatus-plugin-kv - <<'EOF'
path "secret/data/*" {
  capabilities = ["create", "read", "update"]
}
EOF

if [ -n "${KUBERNETES_HOST:-}" ] && [ -n "${TOKEN_REVIEWER_JWT:-}" ] && [ -n "${KUBERNETES_CA_CERT_FILE:-}" ]; then
  if [ ! -f "${KUBERNETES_CA_CERT_FILE}" ]; then
    echo "openbao-seed: KUBERNETES_CA_CERT_FILE missing — skip kubernetes auth" >&2
  else
  bao auth enable kubernetes 2>/dev/null || true
  # disable_iss_validation : Bao hors cluster, issuer kubernetes.default.svc non joint.
  # TokenReview reste le contrôle.
  bao write auth/kubernetes/config \
    kubernetes_host="${KUBERNETES_HOST}" \
    kubernetes_ca_cert=@"${KUBERNETES_CA_CERT_FILE}" \
    token_reviewer_jwt="${TOKEN_REVIEWER_JWT}" \
    disable_iss_validation=true
  bao write auth/kubernetes/role/admit-sign \
    bound_service_account_names=admit-sign \
    bound_service_account_namespaces=aiforall-apparatus \
    policies=apparatus-transit-sign \
    ttl=15m
  bao write auth/kubernetes/role/controller \
    bound_service_account_names=controller \
    bound_service_account_namespaces=aiforall-apparatus \
    policies=apparatus-transit-verify \
    ttl=15m
  # Surface KV : rôle créé, pas câblé sur le monolithe (kv.backend=postgres).
  bao write auth/kubernetes/role/lazaret-kv \
    bound_service_account_names=gateway \
    bound_service_account_namespaces=aiforall-gateway \
    policies=apparatus-plugin-kv \
    ttl=15m
  echo "openbao-seed: kubernetes auth roles admit-sign/controller/lazaret-kv ready"
  fi
fi

echo "openbao-seed: KV secret/ops/token + transit/keys/apparatus-p4-cosign + policies ready"
