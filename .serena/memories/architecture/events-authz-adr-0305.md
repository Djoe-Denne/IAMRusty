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
- Mise à jour 2026-10-04 — migrations aplaties : aucune donnée en production ; `identities` et ses index sont dans l'unique `m20220101_000001_initial_schema.rs`, sans backfill legacy. Migrations incrémentales seulement quand un état persisté devra être préservé. Statut Accepted / réalité Implemented inchangés.
