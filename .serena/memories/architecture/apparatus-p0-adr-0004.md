# Apparatus P0/P3 — ADR-0004

- Canon : `docs/adr/0004-apparatus-capability-gateway.md`. Jalon : P0 contrat / P3 gateway. Statut : **Accepted**. Réalité : **Implemented** (A-DEC 2026-09-20). SuperSède : aucune.
- Décision Accepted inchangée : toute I/O via gateway ; KV plateforme namespacé ; pas de bearer IAM.
- Réalité : gateway = Lazaret T5–T14b ; KV Postgres+Redis ; OpenBao produit T12 ; `kv_purge` sur `component_removed` ; session mTLS optionnelle T11b.
- APP-05 reste Non décidé (options A/B dans 0004, non tranchées). V1 privé (T7). APP-03 / APP-06 restent ouverts.
- Voir le fichier ADR. Ce digest n’est pas le canon.
