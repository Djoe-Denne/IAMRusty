# ADR-0304 digest

- Jalon : AuthN JWT plateforme
- Chemin : `docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md`
- Statut : Accepted
- Réalité : Partial

- Access RS256 + kid opaque + JWKS ; encodeur = clé plateforme au boot.
- Rotate org N+1 (Pending→Active, ancienne Retiring) ; retiring hors JWKS après TTL+60s.
- Probe Transit live (config IAM `transit_url`/`transit_token`) ; cloud BYOKMS / 0309 absents.
- Voir le fichier ADR.