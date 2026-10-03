# Overlay opt-in mesh AuthN — `kind-mesh` (ADR-0308)

**Pas le défaut.** Distinct de [`../kind`](../kind). **Ne pas** l’ajouter à `../kind/kustomization.yaml`.

Le HelmRelease Flux `../kind/helmrelease-envoy-gateway.yaml` reste **commenté** et hors `resources`. Flux n’est pas sur `aiforall-local`. 0308 reste **Accepted / Partial** (staleness non décidé, mesh non défaut, Flux commenté).

## Apply

```bash
# Contexte kind-aiforall-local uniquement. Ne pas viser kind-apparatus-p4-it.
pwsh deploy/deploy-mesh.ps1
bash scripts/mesh-authn-kind-e2e.sh
```

`deploy-mesh.ps1` compile ext-authz dans Docker, `kind load` les images déjà construites (`aiforall-*-service:latest`, ext-authz, postgres, openfga, alpine), régénère les certificats, puis applique le rendu de `kubectl kustomize --load-restrictor LoadRestrictionsNone`. Les ConfigMaps de config sortent du répertoire de l’overlay ; `kubectl apply -k` seul est refusé par le load restrictor. Kind ne compile pas les services.

## Contenu

| Ressource | Rôle |
|---|---|
| `platform-services.yaml` | Postgres in-cluster (sans hostPort 5432), OpenFGA, IAM, Hive, Telegraph, Manifesto. Image locale, `TRUSTED_GATEWAY_SAN=envoy-mesh`, certificat client obligatoire. Remplace les Deployments pause du même nom. `kubectl apply -k deploy/apps/overlays/kind` les remet en pause. |
| `ext-authz` | Check HTTP JWT. JWKS `https://iam.aiforall-platform.svc.cluster.local:8443/iam/.well-known/jwks.json`. |
| `envoy-mesh` | Routes exactes signup, login, JWKS sans Check (`ExtAuthzPerRoute.disabled`). Préfixes `/iam/`, `/hive/`, `/telegraph/`, `/manifesto/`. Pas de filtre JWT Envoy. Admin sur `127.0.0.1:9901`. |

Les SAN DNS `*.aiforall-platform.svc.cluster.local` sont sur les certificats de service. Compose et un pod Kind branché sur le port hôte 5432 restent exclusifs : le `DROP DATABASE … WITH (FORCE)` du compose de base ne voit pas ce Postgres.

La preuve Compose reste `bash scripts/mesh-authn-e2e.sh`. La preuve Kind est `bash scripts/mesh-authn-kind-e2e.sh` : deny sans JWT, allow avec `user_id` / `id` du principal, contournement `mesh-client` refusé.
