#!/bin/sh
set -eu
for q in telegraph-events user-events lazaret-kv-events sentinel-sync-events; do awslocal --endpoint-url=http://127.0.0.1:4566 sqs create-queue --queue-name "$q" >/dev/null; done
