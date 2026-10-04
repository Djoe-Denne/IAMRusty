# Apparatus P0/P1 — ADR-0001

- Mise à jour 2026-10-04 — migrations aplaties, un seul fichier nécessaire pour le moment : `services/Manifesto/migration/src/m20241015_000001_initial_schema.rs` (P1/P2/P3 et outbox). Les chemins incrémentaux sont historiques ; `down` retire le schéma complet. Statut et réalité inchangés.

- Canon : `docs/adr/0001-apparatus-binding-owned-by-manifesto.md`
- Jalon : P0/P1
- Statut : Accepted
- Réalité : Partial — preuves P1 ; Partial maintenu
- Binding = `ProjectComponent` 1:1 ; identité = `project_components.id` ; `source` ∈ `legacy|managed` (liste Accepted inchangée)
- Note Réalité 2026-09-13 : jamais déployé, pas de tenants de production ; backfill legacy livré en code mais inutilisé ; migration active mise de côté — voir ADR-0007 checklist 9. Cette ADR ne SuperSède pas elle-même.
- Voir le fichier ADR. Ce digest n’est pas le canon.
