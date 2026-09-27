# ADR-0307 WorkloadIdentity (events/authz)

- Canon : `docs/adr/0307-workload-identity-port.md`
- Statut : Accepted — Réalité : Implemented
- Port `WorkloadIdentity` + factory `compose_workload_identity` ; adapters OIDC WIF AWS/GCP/Azure dans iam-infra et hive-infra (chemin HTTP Hive s2s + IAM Transit)
- Static si provider absent ; fail-closed si WIF incomplet ; InProcess monolithe sans WIF
- Implemented = adapters WIF HTTP + factory ; pas SPIFFE ; pas X509 mesh (0308)
- Non décidé : SPIRE vs WIF natif
- Voir le fichier ADR.