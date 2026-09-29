---
title: Mesh ext_authz opt-in (ADR-0308)
category: concepts
tags: [architecture, iam, jwt, security, visibility/internal]
sources:
  - docs/adr/0308-mesh-authn-jwt.md
  - docs/adr/0307-workload-identity-port.md
  - deploy/mesh/envoy.yaml
  - ext-authz/src/check.rs
summary: >-
  AuthN mesh = Envoy appelle ext-authz. Accepted / Partial. Strip iss/sub/org,
  recreate iss/sub. mTLS de hop et préfixe X-Principal-* encore ouverts.
provenance:
  extracted: 0.86
  inferred: 0.12
  ambiguous: 0.02
created: 2026-09-29T14:45:00Z
updated: 2026-09-29T14:45:00Z
---

# Mesh ext_authz opt-in (ADR-0308)

Canon : `docs/adr/0308-mesh-authn-jwt.md`. Hub : [[projects/aiforall/decisions/0304-access-jwt-trust]]. Le HTTPS compose T14b est un autre sujet : [[projects/aiforall/concepts/https-platform-mesh]].

`Accepted` (acceptation humaine 2026-09-27) ratifie un service **HTTP ext_authz** branché sur Envoy. Pas le filtre JWT Envoy. Pas de sidecar. `Réalité : Partial`. Pas Implemented.

## Contrat

- Le gateway valide le JWT (alg, kid, signature, typ, iss, aud, exp, iat, nbf si présent, trust scope, statut de clé) puis émet `(iss, sub)`.
- Tout `X-Principal-*` client est supprimé à l’entrée et recréé par le mesh. Jamais de confiance dans les valeurs client.
- L’isolation mTLS entre hops est le point 3, renvoyé au port [[projects/aiforall/decisions/0304-access-jwt-trust|0307]]. 0307 livre le WIF HTTP, pas le maillage X509.
- La staleness max de révocation n’a pas de TTL chiffré (0304 §17).

## Déjà dans le dépôt

- Crate `ext-authz/` : `POST /` et `POST /check`, plus un fallback sur le chemin client. Strip de tout `X-Principal-*` puis recreate `x-principal-iss` / `x-principal-sub`. `EXT_AUTHZ_AUDIENCE` obligatoire. Cache JWKS et poll.
- `UserIdExtractor` reste le défaut des services (Hive, IAM). Le mesh ne le remplace pas.
- Compose : `ext-authz` et `envoy-mesh` ont `profiles: ["mesh"]`. Absents de `docker compose up` plain.
- `deploy/apps/overlays/kind-mesh/` est un apply manuel. Il n’est pas dans `overlays/kind/kustomization.yaml`. Le HelmRelease Flux Envoy Kind reste commenté.
- Envoy (`deploy/mesh/envoy.yaml` et le ConfigMap kind-mesh) : `header_mutation` **avant** ext_authz retire `x-principal-iss`, `x-principal-sub`, `x-principal-org`. `allowed_upstream_headers` ne renvoie que iss et sub. Pas de `request_headers_to_remove` sur la route. Pas de `path_prefix` : `/` concaténé au chemin client donnait `//foo` et un 404 du Check.

## Preuve runtime du 29 sept.

`envoy --mode validate` (`envoyproxy/envoy:v1.31-latest`) : configuration OK. `cargo test -p ext-authz --offline -j 1 --lib` : 8 tests ok. En parallèle, le mmap de `librustycog_framework` peut échouer (os error 1455, fichier de pagination).

Trois POST via le listener Envoy `:10000`, binaire Windows, socat, JWKS de test, HTTP clair :

- Spoof `X-Principal-*` sans JWT → 403, upstream non appelé.
- JWT RS256 valide + spoof iss/sub/org → 200. L’upstream a vu `x-principal-iss: http://127.0.0.1/iam` et le `sub` du jeton. Org client absent.
- JWT seul → mêmes iss/sub recréés.

`x-principal-foo` client est encore arrivé à l’upstream. `header_mutation` n’enlève pas un préfixe.

Cette pile n’est pas `docker compose --profile mesh` (pas d’image `local/build-artifacts`) ni un `kubectl apply` de `kind-mesh`.

## Encore ouvert

- Retirer tout nom `x-principal-*`, pas seulement iss/sub/org, avant ext_authz.
- mTLS obligatoire sur les hops (clair et CA étrangère refusés).
- E2e isoprod : conteneur Linux `ext-authz`, IAM réel pour le JWT et le JWKS, CA `platform-mesh`.
- Mesh par défaut, overlay Flux, TTL de staleness.

## Related

- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]
- [[projects/aiforall/concepts/https-platform-mesh]]
- [[journal/2026-09-29]]
- [[projects/aiforall/references/cursor-history-2026-09]]
