#!/usr/bin/env bash
# TEST ONLY: RSA-8192 PKCS#8 for cfg(test) signing/probe.rs; never issuer authority.
# Source the generated file explicitly to inject IAMRUSTY_TEST_RSA8192_PEM_B64.
# CI continues to use the GitHub Actions secret with that exact name.
# Usage: bash generate-probe-keys.sh [new-private-output-directory]
set -euo pipefail
umask 077
command -v openssl >/dev/null 2>&1 || { echo 'openssl required' >&2; exit 1; }
command -v base64 >/dev/null 2>&1 || { echo 'base64 required' >&2; exit 1; }
[ "$#" -le 1 ] || { echo 'Expected at most one new private output directory' >&2; exit 1; }
if [ "$#" -eq 1 ]; then
  output=$1
  # Exclusive creation: never overwrite/adopt a directory or symlink.
  mkdir -m 700 -- "$output"
else
  output=$(mktemp -d)
fi
key="$output/test8192.pem"
env_file="$output/probe-keys.env"
trap 'rm -f -- "$key"' EXIT
echo 'Generating TEST ONLY RSA-8192 key (may take several minutes)...' >&2
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:8192 -out "$key"
b64=$(base64 < "$key" | tr -d '\r\n')
printf 'export IAMRUSTY_TEST_RSA8192_PEM_B64=%s\n' "$b64" > "$env_file"
unset b64
printf 'Private TEST ONLY injection file: %s\n' "$env_file"
