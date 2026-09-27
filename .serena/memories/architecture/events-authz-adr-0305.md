# ADR-0305 digest

- Jalon : Account / Identity / trust domain
- Chemin : `docs/adr/0305-account-identity-trust-domain.md`
- Statut : Accepted
- Réalité : Implemented

- Identities Platform + OrganizationManaged persistées.
- Login/refresh = `ensure_platform_identity` seulement.
- RPC interne `POST /iam/internal/organizations/{org_id}/identities` ; Hive membership n’appelle pas ce RPC.
- Pas d’UX switch ni second JWT org.
- Voir le fichier ADR.