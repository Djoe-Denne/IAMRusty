#!/bin/sh
# Privileged phase, only inside the new coffer epoch (run-openbao.sh).
set -eu
export BAO_ADDR BAO_TOKEN
until bao status >/dev/null 2>&1; do sleep 1; done
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

bao policy write local-full-lazaret - >/dev/null <<'EOF'
path "auth/token/lookup-self" { capabilities = ["read"] }
path "secret/data/ops/token" { capabilities = ["read"] }
EOF
bao policy write local-full-github-connect - >/dev/null <<'EOF'
path "auth/token/lookup-self" { capabilities = ["read"] }
path "secret/data/github-connect/oauth" { capabilities = ["read"] }
path "secret/data/github-connect/iam" { capabilities = ["read"] }
EOF
bao policy write local-full-iam-hmac - >/dev/null <<'EOF'
path "auth/token/lookup-self" { capabilities = ["read"] }
path "secret/data/github-connect/iam" { capabilities = ["read"] }
EOF
bao policy write local-full-github-seed - >/dev/null <<'EOF'
path "auth/token/lookup-self" { capabilities = ["read"] }
path "secret/data/github-connect/oauth" { capabilities = ["read", "create", "update"] }
path "secret/data/github-connect/iam" { capabilities = ["read", "create", "update"] }
path "secret/metadata/github-connect/oauth" { capabilities = ["read"] }
path "secret/metadata/github-connect/iam" { capabilities = ["read"] }
EOF
bao write auth/kubernetes/role/lazaret bound_service_account_names=lazaret \
  bound_service_account_namespaces=aiforall-local-full audience=openbao-lazaret \
  token_policies=local-full-lazaret token_no_default_policy=true token_type=service \
  token_ttl=8h token_max_ttl=8h token_explicit_max_ttl=8h >/dev/null
bao write auth/kubernetes/role/github-connect bound_service_account_names=github-connect \
  bound_service_account_namespaces=aiforall-local-full audience=openbao-github-connect \
  token_policies=local-full-github-connect token_no_default_policy=true token_type=service \
  token_ttl=15m token_max_ttl=1h token_explicit_max_ttl=1h >/dev/null
bao write auth/kubernetes/role/iam-hmac bound_service_account_names=iam-openbao \
  bound_service_account_namespaces=aiforall-local-full audience=openbao-iam-hmac \
  token_policies=local-full-iam-hmac token_no_default_policy=true token_type=service \
  token_ttl=5m token_max_ttl=5m token_explicit_max_ttl=5m >/dev/null
bao write auth/kubernetes/role/github-seed bound_service_account_names=github-seed \
  bound_service_account_namespaces=aiforall-local-full audience=openbao-github-seed \
  token_policies=local-full-github-seed token_no_default_policy=true token_type=service \
  token_ttl=5m token_max_ttl=5m token_explicit_max_ttl=5m >/dev/null
