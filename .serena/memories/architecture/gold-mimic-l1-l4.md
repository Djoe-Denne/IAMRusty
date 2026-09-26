# Gold mimic L1–L4 (2026-09-26)

Kind `aiforall-local` rapproché d’un mimic prod. Preuve : `just prove-gold` → invoke 200, hostname `plugin-98ba747fc572de29d76dfd08f9253778`.

- **Zot** : `anonymousPolicy: []`, htpasswd signer (write) + reader (read), probe Basic reader, Secret `zot-registry-accounts`.
- **OpenBao** : auth Kubernetes (SA admit-sign / controller), pas de `lazaret-dev-root` dans Job/operator. NP `:8200` pinée (pas `0.0.0.0/0`). Root seulement bootstrap seed.
- **Policy `apparatus-transit-sign`** : sign + hmac + **verify** + read. Le Job Adm-A appelle encore `sign_and_verify` avec le même token ; sans verify Cosign 403. Rôle `controller` reste verify-only.
- **Catalogue** : Bearer `CATALOG_TOKEN=aiforall-gold-catalog` = `MANIFESTO_SERVICE__COMPONENT_SERVICE__API_KEY`. Anonyme 401.
- **Leftover** : `just debt-schedule-reference-kv` seulement ; prove-gold delete best-effort `svc/apparatus-reference-kv`.
- Hors session : ADR-0009/0010, AppRole, OIDC zot, fixtures kind/zot, `apparatus-p4-it`.
