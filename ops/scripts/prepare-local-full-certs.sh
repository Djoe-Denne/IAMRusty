#!/bin/sh
# Linux/CI compatibility entrypoint; PKI logic is shared with native Windows.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "${PYTHON:-python3}" -B "$SCRIPT_DIR/prepare-local-full-certs.py" "$@"
