# Correctifs revue uncommitted Lazaret P3 (2026-09-16)

- Lots 1–3 **et** lot restant (enroll first-wins + tests T7 HTTP) **implémentés**, pas de commit.
- Packages : `manifesto-service`, `lazaret-service`, `lazaret-domain`, `lazaret-infra`.
- Enroll : consult live + réécriture generation/grant_revision au **premier** enroll. **First-wins** : binding déjà enrollé, autre empreinte → `IdentityError::BindingAlreadyEnrolled` HTTP **409** ; même empreinte → no-op. Plus de `retain` last-writer-wins.
- Conséquence : après bump `grant_revision`, le workload ne peut plus se ré-enroller (409). `issue_session` relit le store figé. Recovery = restart processus (registre mémoire) ou rotation future attestée. Pas d’auth sur `/enroll`.
- GET snapshot Manifesto = JWT plateforme. Invoke : Origin Background, pas de query `principal`.
- T7 HTTP : `boot()` → TestFixture Postgres (`has_db=true`). Docker down ⇒ T7 non exécutables ; pas de `#[ignore]`.
- Ne pas relitiger ADR-0007. Pas le mot Lazaret dans Manifesto src.
