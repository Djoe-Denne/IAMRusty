Jalon : architecture actuelle / AuthN s2s.
Chemin : docs/adr/0307-workload-identity-port.md
Statut : Proposed · Réalité : Unimplemented

- Port WorkloadIdentity, pas une dépendance SPIFFE obligatoire de 0304.
- SPIFFE/SPIRE absents du dépôt. Lazaret::WorkloadIdentity = identité plugin Apparatus, pas ce port.
- Ordre credentials : OIDC WIF, X509/mTLS, StaticCredential en dernier (secret OpenBao).

Voir le fichier ADR.
