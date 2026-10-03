---
name: envoy-runtime-debug
description: >-
  Debug runtime Envoy de CE dépôt (Compose mesh + overlay Kind). Parcourt
  client → listener → filter chain → virtual host → route → cluster → endpoint
  → service/pod/container. Admin API en lecture seule. Ne jamais modifier la
  config Envoy avant d'avoir identifié le hop qui échoue.
---

# Envoy runtime debug (AIForAll)

Skill **spécifique à ce repo**. Ne pas généraliser à un Envoy tiers.

## Invariants

- Pour **chaque** requête, parcourir dans cet ordre : client → listener → filter chain → virtual host → route → cluster → endpoint → service / pod / container.
- **Ne jamais modifier** la config Envoy (YAML, ConfigMap, args) **avant** d'avoir identifié le hop qui échoue.
- Admin API **read-only** uniquement. Surface sensible : **loopback**, `kubectl port-forward`, ou `exec` dans le conteneur. **Jamais** exposée publiquement.
- Ne pas publier le port admin. Ne pas changer les YAML listés ci-dessous.

## Fichiers du dépôt (ne pas modifier)

| Fichier | Bind admin |
|---|---|
| `ops/deploy/mesh/envoy.yaml` | `0.0.0.0:9901` (utilisé par Compose mesh) |
| `ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml` | `127.0.0.1:9901` |

Compose : service `envoy-mesh` dans `docker-compose.yml` (profil `mesh`), image `envoyproxy/envoy:v1.31-latest`, volume vers `ops/deploy/mesh/envoy.yaml`. Ports publiés : **`10000:10000` seulement**. L'admin écoute `0.0.0.0:9901` **dans** le conteneur (réseau Docker `aiforall-network`) ; ce port **n'est pas** dans `ports:`. Overlay Kind : admin loopback uniquement.

## Admin API (GET, lecture seule)

Joindre l'admin **sans publier le port** : `docker exec` / réseau compose, ou `kubectl port-forward` / `exec` vers le Pod Envoy Kind.

- `/ready`
- `/server_info`
- `/config_dump`
- `/clusters?format=json`
- `/listeners`
- `/stats`

Comparer **config déclarée** (YAML ci-dessus) vs **config chargée** (`/config_dump`, `/server_info`). Un YAML à jour n'implique pas un process rechargé.

Ne pas démarrer la stack uniquement pour inspecter l'admin. Si aucune instance ne tourne déjà : **INCONCLUSIVE**.

## Checks upstream (cluster → workload)

Après identification du cluster / de la route :

- cluster présent dans `/clusters` et `/config_dump`
- endpoints (active vs empty)
- DNS / résolution du hostname de cluster
- health (active/passive, outlier)
- circuit breakers
- HTTP/1.1 vs HTTP/2 (codec mismatch)
- TLS et SNI vers l'upstream
- Service Kubernetes, EndpointSlice
- Pod Ready, `containerPort` vs `targetPort`

## Checks downstream (client → listener)

- listener bind et port
- filter chain (ALPN, SNI)
- SNI / TLS downstream
- route et virtual host (domaine, path, rewrite)
- headers (Host, `:authority`, auth mesh)
- protocole (HTTP/1.1 vs HTTP/2, mTLS)

## Hop qui échoue — exemples

| Symptôme | Hops à vérifier en premier |
|---|---|
| Connexion refusée | listener bind, containerPort, Service |
| TLS / SNI | filter chain, certificats, SAN |
| 404 Envoy | virtual host, route |
| 503 no healthy upstream | cluster, endpoints, health, Pod |
| timeout | circuit breakers, upstream hang, DNS |
| HTTP/2 PROTOCOL_ERROR | codec H1 vs H2 |

## Sortie attendue

Hop fautif, preuve admin ou `kubectl`/`docker`, config déclarée vs chargée, **aucune** modification YAML tant que le hop n'est pas nommé.
