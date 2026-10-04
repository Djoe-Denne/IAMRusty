#!/bin/sh
# Development-only local OpenBao. No root token is given to the controller.
set -eu
export BAO_ADDR BAO_TOKEN
bao kv put secret/ops/token token=local-full-simulated-vendor >/dev/null
if ! bao secrets list -format=json | grep -q '"transit/"'; then bao secrets enable transit >/dev/null; fi
if ! bao read transit/keys/apparatus-p4-cosign >/dev/null 2>&1; then
  bao write -f transit/keys/apparatus-p4-cosign type=ecdsa-p256 >/dev/null
fi
bao policy write local-full-transit-verify - >/dev/null <<'EOF'
path "transit/verify/apparatus-p4-cosign" { capabilities = ["update"] }
path "transit/keys/apparatus-p4-cosign" { capabilities = ["read"] }
EOF
bao policy write local-full-transit-sign - >/dev/null <<'EOF'
path "transit/sign/apparatus-p4-cosign" { capabilities = ["update"] }
path "transit/verify/apparatus-p4-cosign" { capabilities = ["update"] }
path "transit/keys/apparatus-p4-cosign" { capabilities = ["read"] }
EOF
if ! bao auth list -format=json | grep -q '"kubernetes/"'; then bao auth enable kubernetes >/dev/null; fi
# OpenBao itself uses its projected SA token for TokenReview; reviewer JWT is
# deliberately not copied into the seed Job or fetched from any live Secret.
bao write auth/kubernetes/config kubernetes_host=https://kubernetes.default.svc:443 \
  kubernetes_ca_cert=@/kube-ca/ca.crt disable_local_ca_jwt=false >/dev/null
bao write auth/kubernetes/role/controller bound_service_account_names=controller \
  bound_service_account_namespaces=aiforall-local-apparatus policies=local-full-transit-verify ttl=15m >/dev/null
bao write auth/kubernetes/role/admit-sign bound_service_account_names=admit-sign \
  bound_service_account_namespaces=aiforall-local-apparatus policies=local-full-transit-sign ttl=15m >/dev/null
echo 'local-full OpenBao development mounts/policies configured'
