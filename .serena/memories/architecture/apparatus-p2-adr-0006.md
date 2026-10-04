# Apparatus P2 — ADR-0006

- Mise à jour 2026-10-04 — migrations aplaties, un seul fichier nécessaire pour le moment : `services/Manifesto/migration/src/m20241015_000001_initial_schema.rs` (P1/P2/P3 et outbox). Les chemins incrémentaux sont historiques ; `down` retire le schéma complet. Statut et réalité inchangés.

- Canon : `docs/adr/0006-apparatus-p2-reconciliation-in-process.md`
- Jalon : P2
- Statut : Accepted
- Réalité : Implemented — T1–T7 et writer backoff §D livrés ; aucun écart P2 ouvert
- Contrôleur in-process Manifesto (ticker + scan DB) : génération CAS commande, lease 30 s, fencing génération+epoch ; pas de K8s, pas de HTTP 202, pas de nouvel event
- M (consentement) reste Hors P2. Note Réalité 2026-09-13 : consentement capacités = P3 (ADR-0007 item 9) ; jamais déployé, pas de tenants ; legacy→managed n’est pas un travail actif ; `source` ∈ legacy|managed inchangé. Cette ADR ne SuperSède pas elle-même.
- Voir le fichier ADR. Ce digest n’est pas le canon.
