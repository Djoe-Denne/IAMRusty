Jalon : architecture actuelle / AuthN s2s & WIF.
Chemin : docs/adr/0307-workload-identity-port.md
Statut : Proposed · Réalité : Unimplemented

- Port WorkloadIdentity ; SPIFFE évalué, **pas** dépendance obligatoire de 0304.
- SPIFFE/SPIRE ABSENT du runtime. WIF (OIDC/X509) préféré ; pas de service-account-key.json nominal.
- Accept humain requis. StaticCredential = fallback OpenBao seulement.

Voir le fichier ADR.