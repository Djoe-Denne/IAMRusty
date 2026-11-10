#!/bin/sh
set -eu
umask 077
test -z "$(ls -A /runtime)"
test -z "$(ls -A /data)"
cp /inputs/config.json /inputs/ca.crt /inputs/server.crt /inputs/server.key /runtime/
chmod 700 /runtime /data
chmod 400 /runtime/*
chown -R 10001:10001 /runtime /data
