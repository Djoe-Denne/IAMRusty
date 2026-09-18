# Apparatus P3 — ADR-0007

- Jalon P3. Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Statut : **Accepted**. Réalité : **Partial**. SuperSède : aucune.
- Décision : BC **Lazaret** (`Lazaret` / `lazaret-service`) = frontière capacités/données, distinct de Manifesto. 0006 G et E restent (pas d'`invoke` sur `ApparatusRuntime` Manifesto ; zéro `gateway` sous `Manifesto/*/src` ; `/components` gelé à 5).
- T12 : trou OpenBao produit hors compose **fermé** (compose pin `openbao/openbao:2.6.2` + testcontainer service-local ; IT `apparatus_p3_t12_openbao`). T6 IT reste wiremock. T11b mTLS `/session` fermé. T1–T10 restent livrés.
- Holes (bloquent Implemented) : APP-05 ; G/E Manifesto ; pas K8s ; pas de second protocole ; compose TLS / mTLS Hive-IAM-Telegraph / persistance clé CA. **Pas** Implemented.
- Voir le fichier ADR. Ce digest n’est pas le canon.
