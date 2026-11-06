# Overlay opt-in mesh AuthN — `kind-mesh` (ADR-0308)

**Pas le défaut.** Distinct de [`../kind`](../kind). **Ne pas** l’ajouter à `../kind/kustomization.yaml`.

Le HelmRelease Flux `../kind/helmrelease-envoy-gateway.yaml` reste **commenté** et hors `resources`. Flux n’est pas sur `aiforall-local`. 0308 reste **Accepted / Partial** (staleness non décidé, mesh non défaut, Flux commenté).

## Apply

```bash
# Contexte kind-aiforall-local uniquement. Ne pas viser kind-apparatus-p4-it.
pwsh ops/deploy/deploy-mesh.ps1
```

`deploy-mesh.ps1` compile ext-authz dans Docker, `kind load` les images déjà construites (`aiforall-*-service:latest`, ext-authz, postgres, openfga, alpine), régénère les certificats, puis applique le rendu de `kubectl kustomize --load-restrictor LoadRestrictionsNone`. Les ConfigMaps de config sortent du répertoire de l’overlay ; `kubectl apply -k` seul est refusé par le load restrictor. Kind ne compile pas les services.

## Contenu

| Ressource | Rôle |
|---|---|
| `platform-services.yaml` | Postgres in-cluster (sans hostPort 5432), OpenFGA, IAM, Hive, Telegraph, Manifesto. Image locale, `TRUSTED_GATEWAY_SAN=envoy-mesh`, certificat client obligatoire. Remplace les Deployments pause du même nom. `kubectl apply -k ops/deploy/apps/overlays/kind` les remet en pause. |
| `ext-authz` | Check HTTP JWT. JWKS `https://iam.aiforall-platform.svc.cluster.local:8443/iam/.well-known/jwks.json`. |
| `envoy-mesh` | Routes exactes signup, login, JWKS sans Check (`ExtAuthzPerRoute.disabled`). Préfixes `/iam/`, `/hive/`, `/telegraph/`, `/manifesto/`. Pas de filtre JWT Envoy. Admin sur `127.0.0.1:9901`. |

Les SAN DNS `*.aiforall-platform.svc.cluster.local` sont sur les certificats de service. Compose et un pod Kind branché sur le port hôte 5432 restent exclusifs : le `DROP DATABASE … WITH (FORCE)` du compose de base ne voit pas ce Postgres.

Les anciens lanceurs E2E shell ont été retirés le 6 novembre 2026 à la demande de
l'utilisateur. Le seul circuit E2E conservé est [k6](../../../../tests/k6/AUTH-WINDOWS-PARENT.md).
Leur retrait ne qualifie pas cet overlay avec k6 et ne prouve pas le portage des
anciens cas mesh ; le raccord au profil GitHub seul reste à établir.
