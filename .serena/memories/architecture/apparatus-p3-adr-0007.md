# Apparatus P3 — ADR-0007

- Jalon P3. Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Statut : **Accepted**. Réalité : **Partial**. SuperSède : aucune.
- Décision : BC **Lazaret** (`Lazaret` / `lazaret-service`) = frontière capacités/données, distinct de Manifesto. 0006 G et E restent (pas d'`invoke` sur `ApparatusRuntime` Manifesto ; zéro `gateway` sous `Manifesto/*/src` ; `/components` gelé à 5).
- T13 : trou persistance clé CA **fermé** + trou compose TLS **Lazaret** **fermé** (HTTPS `tls_port` 8080, volume certs, HEALTHCHECK `curl -fk` ; IT `apparatus_p3_t13_ca_persist`). T12 OpenBao produit fermé. T11b mTLS `/session` fermé. T1–T10 restent livrés.
- Holes (bloquent Implemented) : APP-05 ; G/E Manifesto ; pas K8s ; pas de second protocole ; mTLS Hive-IAM-Telegraph. **Pas** Implemented.
- Voir le fichier ADR. Ce digest n’est pas le canon.
