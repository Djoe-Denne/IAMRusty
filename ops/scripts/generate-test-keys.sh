#!/usr/bin/env bash
# Régénère la paire de clés RSA-8192 TEST-ONLY utilisée par le test de sonde
# d'admission (services/IAMRusty/infra/src/signing/probe.rs).
#
# - Sortie : variable IAMRUSTY_TEST_RSA8192_PEM_B64 (PEM PKCS#8 en base64)
#   ajoutée à .env à la racine du repo (gitignored).
# - CI : la même valeur vit dans le secret GitHub Actions du même nom.
# - AUCUNE AUTORITÉ : le trust est lié à l'issuer/JWKS publié par environnement,
#   pas à ce fichier. Clé de test, jamais utiliser ailleurs.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
command -v openssl >/dev/null 2>&1 || { echo "openssl requis" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Génération RSA-8192 (plusieurs minutes possibles)..."
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:8192 -out "$tmp/test8192.pem"
b64="$(base64 -w0 "$tmp/test8192.pem")"

{
  echo "# Généré par ops/scripts/generate-test-keys.sh le $(date -u '+%Y-%m-%dT%H:%MZ') — TEST ONLY, aucune autorité"
  echo "IAMRUSTY_TEST_RSA8192_PEM_B64=${b64}"
} >> "$root/.env"

echo "Ajouté à .env (gitignored — ne jamais committer)."
