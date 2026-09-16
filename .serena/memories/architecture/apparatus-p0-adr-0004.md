# Apparatus P0/P3 — ADR-0004

- Canon : `docs/adr/0004-apparatus-capability-gateway.md`. Jalon : P0 contrat / P3 gateway. Statut : **Accepted**. Réalité : **Partial**. SuperSède : aucune.
- Décision Accepted inchangée : toute I/O via gateway ; KV plateforme namespacé ; pas de bearer IAM.
- Réalité plus proche : gateway réseau = Lazaret `POST /invoke` + connecteurs nommés (pas Manifesto) ; KV Postgres+Redis ; secrets-by-ref + Vault HTTP (IT wiremock).
- Pas Implemented : mTLS rustls complete résiduel, OpenBao produit hors compose, `kv_purge` non branché unbind Manifesto.
- Voir le fichier ADR. Ce digest n’est pas le canon.
