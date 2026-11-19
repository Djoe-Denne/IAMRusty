#!/usr/bin/env bash
# Only for the hosted IAM timing experiment. Each mirror gets fresh indexes and
# an empty archive cache. Do not fall back after starting a package installation.
set -euo pipefail

mode=${1:?expected host or runtime}
shift
case "$mode" in
  host)
    destination=${1:?expected the fresh host package staging directory}
    packages=(libcurl4-openssl-dev libcurl4t64)
    ;;
  runtime)
    packages=(ca-certificates curl libssl3t64)
    ;;
  *) exit 2 ;;
esac

[[ $(id -u) == 0 && $(dpkg --print-architecture) == amd64 ]]
. /etc/os-release
[[ "$ID" == ubuntu && "$VERSION_CODENAME" == noble ]]
umask 022
ca=${IAM_APT_CA:-/etc/ssl/certs/ca-certificates.crt}
[[ -r "$ca" ]]
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
chmod 755 "$work"

for mirror in canonical mit; do
  case "$mirror" in
    canonical)
      archive=https://archive.ubuntu.com/ubuntu
      security=https://security.ubuntu.com/ubuntu
      ;;
    mit)
      archive=https://mirrors.mit.edu/ubuntu
      security=$archive
      ;;
  esac
  attempt=$work/$mirror
  mkdir -p "$attempt/lists" "$attempt/archives"
  cat > "$attempt/sources.list" <<EOF
deb [arch=amd64 signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] $archive noble main
deb [arch=amd64 signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] $archive noble-updates main
deb [arch=amd64 signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] $security noble-security main
EOF
  apt_options=(
    -o "Dir::Etc::sourcelist=$attempt/sources.list" -o Dir::Etc::sourceparts=-
    -o "Dir::State::lists=$attempt/lists" -o "Dir::Cache::archives=$attempt/archives"
    -o Dir::Cache::pkgcache= -o Dir::Cache::srcpkgcache=
    -o Acquire::Retries=1 -o Acquire::https::Timeout=15 -o "Acquire::https::CaInfo=$ca"
    -o Acquire::Languages=none
  )
  if timeout -k 5s 65s apt-get "${apt_options[@]}" update --error-on=any &&
     timeout -k 5s 55s apt-get "${apt_options[@]}" install -y --no-install-recommends --no-remove \
       --reinstall --download-only "${packages[@]}"; then
    echo "IAM APT source selected: $mirror"
    if [[ "$mode" == host ]]; then
      install -m 0644 -- "$attempt"/archives/*.deb "$destination/"
    else
      timeout -k 5s 90s apt-get "${apt_options[@]}" install -y --no-install-recommends --no-remove \
        --no-download "${packages[@]}"
      dpkg-query -W -f='${Package}=${Version}\n' "${packages[@]}"
    fi
    exit 0
  fi
  echo "IAM APT preparation failed at $mirror" >&2
done
exit 1
