#!/bin/sh
# Volatile local/CI coffer only. Never relay the DEV banner (root + unseal key).
set -eu
umask 077
unset VAULT_TOKEN VAULT_DEV_ROOT_TOKEN_ID BAO_TOKEN
export BAO_ADDR=http://127.0.0.1:8200
export BAO_CLIENT_TIMEOUT=5s
BAO_TOKEN=$(od -An -N32 -tx1 /dev/urandom | tr -d ' \n')
[ "${#BAO_TOKEN}" -eq 64 ] || exit 1
export BAO_TOKEN
rm -f /run/openbao/bootstrap-ready
VAULT_DEV_ROOT_TOKEN_ID="$BAO_TOKEN" bao server -dev -dev-no-store-token \
  -dev-listen-address=127.0.0.1:8200 -config=/scripts/openbao.hcl >/dev/null 2>&1 &
server=$!
cleanup() {
  trap - EXIT TERM INT
  kill -TERM "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
}
trap cleanup EXIT TERM INT
if ! timeout 240 sh /scripts/seed-openbao.sh >/dev/null 2>&1; then
  echo 'OpenBao private bootstrap failed; server stopped (diagnostics suppressed)' >&2
  exit 1
fi
bao token revoke -self >/dev/null 2>&1 || exit 1
refusal=$(bao token lookup 2>&1) && exit 1
case "$refusal" in *'Code: 403'*) ;; *) exit 1 ;; esac
unset BAO_TOKEN refusal
bao status >/dev/null 2>&1 || exit 1
touch /run/openbao/bootstrap-ready
echo 'OpenBao local epoch bootstrapped; bootstrap root revoked'
wait "$server"
