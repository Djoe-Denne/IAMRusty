#!/bin/sh
# Runs only in the private new-coffer bootstrap, before root revocation.
# Rights/configuration are public HCL/JSON documents, not shell service tables.
set -eu
export BAO_ADDR BAO_TOKEN
until bao status >/dev/null 2>&1; do sleep 1; done
for file in /scripts/openbao-mount-*.json; do
  name=${file##*/}; name=${name#openbao-mount-}; name=${name%.json}
  if ! bao secrets list -format=json | grep -q "\"$name/\""; then
    bao write "sys/mounts/$name" - < "$file" >/dev/null
  fi
done
for file in /scripts/openbao-auth-*.json; do
  name=${file##*/}; name=${name#openbao-auth-}; name=${name%.json}
  if ! bao auth list -format=json | grep -q "\"$name/\""; then
    bao write "sys/auth/$name" - < "$file" >/dev/null
  fi
done
set --
while IFS= read -r argument; do set -- "$@" "$argument"; done < /scripts/openbao-auth-config.args
bao write "$(cat /scripts/openbao-auth-config.path)" "$@" >/dev/null
for file in /scripts/openbao-request-*.json; do
  path=$(cat "${file%.json}.path")
  if ! bao read "$path" >/dev/null 2>&1; then bao write "$path" - < "$file" >/dev/null; fi
done
for file in /scripts/openbao-policy-*.hcl; do
  name=${file##*/}; name=${name#openbao-policy-}; name=${name%.hcl}
  bao policy write "$name" "$file" >/dev/null
done
for file in /scripts/openbao-role-*.json; do
  name=${file##*/}; name=${name#openbao-role-}; name=${name%.json}
  bao write "auth/kubernetes/role/$name" - < "$file" >/dev/null
done
# Business fixture seed, intentionally separate from public permission documents.
bao kv put secret/ops/token token=local-full-simulated-vendor >/dev/null
