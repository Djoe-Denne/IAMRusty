Jalon : architecture actuelle / AuthN identité.
Chemin : docs/adr/0305-account-identity-trust-domain.md
Statut : Accepted · Réalité : Unimplemented

- HumanAccount = UX ; Identity = un trust domain ; principal = (iss, sub). 1 compte → N identities.
- Platform-managed : une PlatformIdentity, plusieurs orgs, pas de JWT par org. Organization-managed / BYOKMS : identity distincte par domain ; switch explicite ≠ changer d’org OpenFGA.
- Hive = SoT membership (devra porter (iss, sub)). Runtime : OrganizationMember.user_id. IAM = comptes, sessions, SigningKeyRegistry, JWKS.
- Révocation membership ≠ révocation de clé. auth_version / session_version = Non décidé.

Voir le fichier ADR.