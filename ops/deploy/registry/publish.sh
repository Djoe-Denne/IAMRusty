#!/bin/sh
set -eu
umask 077
# Destination is a validated, non-secret OCI reference, never evaluated as shell.
IFS= read -r destination < /inputs/destination
exec /usr/bin/skopeo --policy /scripts/policy.json copy --format oci \
  --dest-tls-verify=true --dest-cert-dir /certs --digestfile /outputs/digest \
  docker-archive:/inputs/image.tar "$destination"
