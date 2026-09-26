# Contrat d'outputs cluster — tranche locale 0603

Module : `cloud/opentofu/modules/cluster/`

Outputs (ADR-0600, identiques pour tout adaptateur futur) :

- `cluster_name`
- `kubernetes_host`
- `cluster_ca`
- `oidc_issuer` (nullable)
- `auth_exec`

Pas de provider `google`. Pas de stack `cloud/opentofu/live/.../gcp/` dans cette tranche : **aucun apply GKE**.

Valider sans cloud ni credentials :

```
tofu -chdir=cloud/opentofu/modules/cluster init -backend=false
tofu -chdir=cloud/opentofu/modules/cluster validate
```

Si `tofu` / `terraform` est absent, M1 ne échoue pas (`just deploy-m1` n'exige pas OpenTofu). Kind n'est pas un adapter OpenTofu (0600 : kind via CLI + `just`).
