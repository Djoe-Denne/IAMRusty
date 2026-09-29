# Overlay opt-in mesh AuthN — `kind-mesh` (ADR-0308)

**Pas le défaut.** Distinct de [`../kind`](../kind). **Ne pas** l’ajouter à `../kind/kustomization.yaml`.

Le HelmRelease Flux `../kind/helmrelease-envoy-gateway.yaml` reste **commenté** et hors `resources` (Flux n’est pas sur `aiforall-local`).

## Apply manuel

```bash
# Image locale (après docker build du bin ext-authz)
kind load docker-image aiforall-ext-authz:local --name aiforall-local

kubectl apply -k deploy/apps/overlays/kind-mesh
```

## Contenu

| Ressource | Rôle |
|---|---|
| `ext-authz` Deployment/Service | Check HTTP JWT → `x-principal-iss` / `x-principal-sub` |
| `envoy-mesh` ConfigMap/Deployment/Service | Envoy `http_service` ext_authz (pas filtre JWT Envoy) |

`EXT_AUTHZ_JWKS_URL` pointe vers `iam.aiforall-platform` (stub pause en base Kind tant que l’IdP réel n’est pas câblé).

## Preuve E2E sans Docker / Kind

Le contrat Check (strip spoof + recreate iss/sub) est prouvé par le harness cargo **stand-in** :

```bash
cargo test -p ext-authz --test mesh_path_standin
```

Ce harness **n’affirme pas** qu’Envoy a tourné. JWT in-process / `UserIdExtractor` reste le défaut des services (0308 **Partial**, pas Implemented).
