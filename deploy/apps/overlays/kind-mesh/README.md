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

`EXT_AUTHZ_JWKS_URL` est `https://iam.aiforall-platform:8443/iam/.well-known/jwks.json`. Le secret `platform-mesh-certs` (mêmes fichiers que `certs/platform-mesh`) est monté par Envoy et ext-authz. mTLS, certificat client obligatoire.

La preuve isoprod rejouable est `bash scripts/mesh-authn-e2e.sh` (Compose, pas Kind). Le stand-in `cargo test -p ext-authz --test mesh_path_standin` ne prouve pas qu’Envoy a tourné. 0308 reste **Partial**.
