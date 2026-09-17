# Apparatus P3 — ADR-0007

- Jalon P3. Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Statut : **Accepted**. Réalité : **Partial**. SuperSède : aucune.
- Décision : BC **Lazaret** (`Lazaret` / `lazaret-service`) = frontière capacités/données, distinct de Manifesto. 0006 G et E restent (pas d'`invoke` sur `ApparatusRuntime` Manifesto ; zéro `gateway` sous `Manifesto/*/src` ; `/components` gelé à 5).
- T1–T9 : absence, gate+scaffold, identité hybride, consult live, consent write, KV Postgres+Redis + secrets-by-ref, invoke HTTP + connecteurs nommés, `kv_purge` sur `component_removed`, T9 chemin public `POST /lazaret/invoke` sur `prefixed_router` (`INVOKE_PATH` reste `/invoke` ; `Application::router()` reste non préfixé).
- Holes (bloquent Implemented) : mTLS rustls complete / enrollment T3 in-memory ; OpenBao produit hors compose ; APP-05 ; G/E Manifesto ; pas K8s ; pas de second protocole. Hole `kv_purge` fermé (T8). Pas de hole de chemin invoke.
- Voir le fichier ADR. Ce digest n’est pas le canon.
