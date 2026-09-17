# Apparatus P3 — ADR-0007

- Jalon P3. Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Statut : **Accepted**. Réalité : **Partial**. SuperSède : aucune.
- Décision : BC **Lazaret** (`Lazaret` / `lazaret-service`) = frontière capacités/données, distinct de Manifesto. 0006 G et E restent (pas d'`invoke` sur `ApparatusRuntime` Manifesto ; zéro `gateway` sous `Manifesto/*/src` ; `/components` gelé à 5).
- T1–T7 : absence, gate+scaffold, identité hybride, consult live, consent write, KV Postgres+Redis + secrets-by-ref, invoke HTTP + connecteurs nommés.
- P3-close / T8 : `kv_purge` branché sur Manifesto `component_removed` via file dédiée `lazaret-kv-events` / `test-lazaret-kv-events`. Handler purge only ; pas de réplique grants ; pas de crate `lazaret-events`.
- Holes (bloquent Implemented) : mTLS rustls complete / enrollment T3 in-memory ; OpenBao produit hors compose ; invoke IT `/invoke` vs nest `/lazaret` ; APP-05 ; G/E Manifesto ; pas K8s ; pas de second protocole.
- Voir le fichier ADR. Ce digest n’est pas le canon.
