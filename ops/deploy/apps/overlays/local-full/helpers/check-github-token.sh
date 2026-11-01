#!/bin/sh
set -eu
# Startup oracle only. Never prints the sink, CLI output or an authentication error.
export BAO_ADDR=https://openbao.aiforall-local-full.svc.cluster.local:8443
export BAO_CACERT=/run/openbao-ca/ca.crt
export BAO_CLIENT_TIMEOUT=5s
BAO_TOKEN=$(cat /run/github-openbao/token)
export BAO_TOKEN
ttl=$(bao token lookup -field=ttl 2>/dev/null)
case "$ttl" in ''|*[!0-9]*) exit 1 ;; esac
[ "$ttl" -ge 30 ] && [ "$ttl" -le 3600 ]
policies=$(bao token lookup -field=policies 2>/dev/null)
case "$policies" in *root*) exit 1 ;; esac
