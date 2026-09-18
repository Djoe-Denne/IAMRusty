# Apparatus P3 — ADR-0007

- Jalon P3. Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Statut : **Accepted**. Réalité : **Partial**. SuperSède : aucune.
- Décision : BC **Lazaret** (`Lazaret` / `lazaret-service`) = frontière capacités/données, distinct de Manifesto. 0006 G et E restent (pas d'`invoke` sur `ApparatusRuntime` Manifesto ; zéro `gateway` sous `Manifesto/*/src` ; `/components` gelé à 5).
- T11b : trou mTLS `/session` **fermé** (HTTPS live + authentification client optionnelle rustycog ; mapping `PeerClientCertificate` → `VerifiedClientCertificate`). T1–T10 restent livrés.
- Holes (bloquent Implemented) : OpenBao produit hors compose ; APP-05 ; G/E Manifesto ; pas K8s ; pas de second protocole ; compose TLS / mTLS Hive-IAM-Telegraph / persistance clé CA. Holes fermés : `kv_purge` (T8), enrollment in-memory (T10), invoke préfixé (T9), mTLS `/session` (T11b).
- Voir le fichier ADR. Ce digest n’est pas le canon.
