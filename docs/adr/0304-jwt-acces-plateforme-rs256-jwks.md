# ADR-0304 : Access JWT = RS256 + kid opaque + JWKS ; SigningProvider ; issuer par trust domain

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Jalon concerné : architecture actuelle / AuthN JWT (complète 0302 ; hors P-Apparatus)
- SuperSède : la décision antérieure **de cette ADR** « OpenBao ne signe pas les JWT utilisateurs » ; **pour la cible seulement**, la décision [0302](0302-authn-jwt-authz-openfga.md) « `iss=iamrusty` unique » (pas l’ADR 0302 entière — AuthZ OpenFGA, Bearer, `aud=aiforall` restent)
- SuperSédée par : —
- Related : [0302](0302-authn-jwt-authz-openfga.md), [0305](0305-account-identity-trust-domain.md), [0306](0306-hive-iam-configuration-signature.md), [0307](0307-workload-identity-port.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md), [0400](0400-iamrusty-identite-hexagonale.md)

`Accepted` ratifie la cible crypto / JWKS / issuer par trust domain. `Réalité : Partial` : mint RS256 + JWKS + extracteur rustycog RS256 + rotate N+1 + probe Transit ; **pas** d’adapters cloud BYOKMS ; remote signer ([0309](0309-remote-signer.md)) absent.

## Contexte

[0302](0302-authn-jwt-authz-openfga.md) fige AuthN Bearer + AuthZ OpenFGA, `iss=iamrusty`, `aud=aiforall`, et laissait ouvert RS256 vs HS256. Avant cette ADR, IAM signait HS256 et `UserIdExtractor` ne vérifiait que HS256. Un HMAC partagé était la dette de rotation. La version antérieure de **cette** ADR interdisait OpenBao Transit pour signer les JWT utilisateurs — remplacée ici.

## Décision

1. **Access tokens user = RS256 + `kid` opaque + JWKS.** HS256 access **interdit après migration**. Fenêtre dual-verify bornée : RS256 = clé RSA + `kid` requis ; HS256 = ancienne clé HMAC, migration only. Pas de try-all-algorithms ; jamais une pubkey RSA comme secret HMAC. Après cutoff : `allowed_algorithms=[RS256]`.
2. **Quatre rôles.** (a) **Émetteur logique** = IAMRusty (flux normal) : choisit principal, trust domain, claims + header + signing input, choisit `SigningKey`, demande la signature, assemble le JWT. (b) **Propriétaire / admin de la clé privée** = plateforme **ou** organisation. (c) **Opération crypto** = `SigningProvider`. (d) **IAM publie le JWKS canonique** `GET /iam/.well-known/jwks.json`. Les consommateurs ne connaissent pas OpenBao / AWS / GCP / Azure / Scaleway / HSM.
3. **KMS uniquement sur le chemin LOGIN / REFRESH** (IAM → SigningProvider → KMS). Jamais sur le chemin requête API. Failure domain : KMS ACME down ⇒ pas de nouveaux tokens / refresh ACME ; JWT ACME déjà émis restent vérifiables ; autres orgs + plateforme OK.
4. **OpenBao Transit EST un SigningProvider autorisé** (Sign, clé non exportée). Mode PEM chargé par IAM autorisé pour dev / simple — compromis documenté (clé en mémoire, éventuellement Secret k8s). **Remplace** « OpenBao ne signe pas ».
5. **Clé dédiée par org platform-managed** (ex. OpenBao `platform-key`, `org-acme-key`, …). Révocation d’une clé org n’impacte pas les autres. **Clé plateforme commune** = fallback orgs sans clé dédiée + identités platform-managed.
6. **BYOKMS** : l’org peut garder la propriété (AWS KMS, GCP KMS, Azure KV / Managed HSM, Scaleway, OpenBao client, HSM, KMIP, PKCS#11, remote signer). IAM construit le signing input ; le KMS signe ; IAM assemble. **Limitation acceptée** : org propriétaire de son KMS = autorité crypto de **son** trust domain (peut produire ses propres principals de ce domain). Elle ne peut pas faire accepter un principal platform, ni d’une autre org, ni un autre issuer. Le KMS ne prouve pas que le digest vient d’IAM — seulement qu’un principal est autorisé à signer.
7. **`aud` reste `aiforall`.** `aud` ≠ organisation. Audiences user-plane / admin / billing = Non décidé.
8. **Trust domain par clé** : `platform` | `organization(org_id)`. Registry conceptuelle `SigningKey` : `id`, `kid` opaque, `algorithm`, `trust_scope`, `issuer`, `provider_type`, `provider_key_ref`, `credential_ref` optionnel, `public_key`, `status` (`pending` | `active` | `retiring` | `revoked`), timestamps. `kid` = **public opaque** — jamais org id, ARN, project GCP, resource Azure, chemin OpenBao, URL.
9. **Issuer lié au trust domain.** Cible, sur le même host que le JWKS (`GET /iam/.well-known/jwks.json`) : platform `https://{host}/iam` ; org `https://{host}/iam/orgs/{slug}`. Le préfixe runtime est `/iam`, pas un sous-domaine `iam.<domain>`. Propriété normative : `kid` → trust domain → issuer. Signature valide + mauvais issuer = **REJECT**. SuperSède **explicitement uniquement** la décision 0302 « `iss=iamrusty` unique » **pour la cible**. Runtime RS256 : `iss={public_base_url}/iam`. Tokens historiques `iss=iamrusty` = mode platform test/HS256 fenêtre jusqu’au cutoff.
10. **Principal canonique = `(iss, sub)`**, jamais `sub` seul.
11. **Claim `org` = contexte de trust, pas permission.** Cohérence `key.owner == org == issuer` du domain = AuthN. OpenFGA = AuthZ. Pas d’org dans `aud`.
12. **Jeton platform-managed** : `iss` platform ; pas forcément réémission par org métier. **Jeton organization-managed** : `iss` org, claim `org`, signé seulement par une clé de ce trust domain.
13. **Rotation = deux durées.** (1) Prépublication N+1 dans le JWKS **avant** de signer avec N+1 (caches découvrent N+1). (2) Rétention N pendant la vie restante des access + clock skew, puis retrait. Pas une seule formule.
14. **Révocation urgence** : `status=revoked`, stop signatures, retrait confiance, propagation validators. Delete JWKS ≠ instantané (cache). Propriété : propagation maximale documentée ; mécanisme détaillé = [0308](0308-mesh-authn-jwt.md). Ne pas confondre révocation clé vs session.
15. **Refresh** : opaques, rotatifs, révocables. Révoquer un refresh n’annule pas les access JWT déjà émis. Pas de deny-list `jti` ici.
16. **Headers `jku`, `x5u`, `jwk`** ignorés / refusés comme trust roots. `kid` ne construit jamais URL / path / host.
17. **Cache JWKS** : pas de fetch par requête ; `kid` connu = validation locale ; `kid` inconnu = refresh coalescé (singleflight), negative cache, last-known-good, refresh périodique. Détail mesh = [0308](0308-mesh-authn-jwt.md). Consommateurs ne parlent pas au KMS. Amendement 2026-10-02, chiffres dans 0308 point 6 : refresh 60 s en staging / prod (configurable), 2 s en local / Kind / tests ; staleness max après révocation = 60 s.
18. **`typ` cible = `aiforall-access+jwt`** (rien dans le runtime ne prouve `at+jwt`). Décision, pas implémenté.
19. **Port conceptuel `SigningProvider`** (`sign_digest`, `public_key`, `capabilities`) ; algo domaine RS256 ; mapping vendor dans l’adapter. Séparer `CredentialProvider` / `WorkloadIdentity` ([0307](0307-workload-identity-port.md)). WIF (OIDC / X509) préféré. `StaticCredential` = fallback ; secret dans OpenBao ; jamais Hive DB ; jamais domain event. Ordre : OIDC WIF, X509/mTLS, static.
20. **JWKS unique** acceptable maintenant ; limite ~1000+ à surveiller ; 10000+ ⇒ évolution sharding / JWKS par issuer. Pas de surconception.
21. **SPIFFE** : pas dépendance obligatoire. Renvoi [0307](0307-workload-identity-port.md). Le domaine dépend d’un port `WorkloadIdentity`.
22. **Remote signer** : contrat minimal → [0309](0309-remote-signer.md).
23. **`IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md` n’est pas canon.**

## État runtime

Access JWT = RS256 + `kid` + `typ=aiforall-access+jwt` via `PemSigningProvider` / Transit adapter ; `iss` = `{public_base_url}/iam` ; `aud=aiforall`. JWKS = `SigningKeyRegistry::list_jwks_keys` (pending+active+retiring ; retiring hors fenêtre après TTL access + skew 60s ; fallback bootstrap cache). Rotate org = N+1 (Pending puis Active / ancienne Retiring) ; probe Transit = `sign_digest` + verify PKCS1v15 (URL Transit depuis config IAM, refuse fermé sans URL). `UserIdExtractor` vérifie RS256 + JWKS et expose `JwtPrincipal` (défaut in-process). En overlay mesh, le service peut au contraire faire confiance au principal passerelle sans revérifier le JWT ([0308](0308-mesh-authn-jwt.md) §7). Runtime TOMLs : `allowed_algorithms=["RS256"]`. HS256 = uniquement le flag explicite `allowed_algorithms` dans les `test.toml` (pas le défaut). Adapters AWS/GCP/Azure BYOKMS **absents**. Remote signer ([0309](0309-remote-signer.md)) **absent**.

## Migration

Dual-verify bornée (RS256 + `kid` ; HS256 = HMAC migration only dans test.toml) puis cutoff HS256 (`allowed_algorithms=[RS256]`). Tokens `iss=iamrusty` = platform historique jusqu’au cutoff issuer. Ne jamais publier le HMAC dans le JWKS.

## Conséquences

- Plus de secret HMAC d’accès partagé entre services en runtime RS256-only ; rotation = JWKS + `kid` ; issuer = trust domain.
- AuthZ OpenFGA et Bearer restent 0302. Account / membership / config signer = 0305 / 0306.
- Consommateurs ne parlent jamais au KMS ; IAM seul publie le JWKS.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| OpenBao ne signe jamais les JWT (ancienne 0304) | Transit Sign + clé non exportée est un SigningProvider légitime ; PEM reste ok pour dev |
| Dual-run HS256+RS256 permanent | Dette HMAC ; bascule bornée puis cutoff |
| HMAC dans le JWKS | Contredit RS256 |
| `aud` = organisation | AuthZ = OpenFGA ; `aud` reste `aiforall` |
| KMS sur chaque requête API | Failure domain et latence inacceptables |
| `kid` = org id / ARN / chemin coffre | Fuite d’infra ; `kid` opaque public seulement |
| Issuer sur un sous-domaine `iam.<domain>` | Le service et le JWKS sont déjà sous le préfixe `/iam` du host gateway |
| SPIFFE obligatoire pour signer | Port WorkloadIdentity (0307) ; pas de dépendance SPIFFE |
| Account + Mesh + SPIFFE dans cette ADR | Périmètre crypto JWT seulement |

## Non décidé ici

- Audiences user-plane / admin / billing.
- Deny-list `jti` / `auth_version` / `session_version` (ADR future).
- Push de révocation (le poll et les durées sont décidés dans [0308](0308-mesh-authn-jwt.md) point 6, 2026-10-02).
- Contrat remote signer détaillé ([0309](0309-remote-signer.md)).
- UX switch d’identity / membership `(iss, sub)` ([0305](0305-account-identity-trust-domain.md), [0306](0306-hive-iam-configuration-signature.md)).

## Références

- Complète : [0302](0302-authn-jwt-authz-openfga.md) (AuthN/AuthZ ; ne SuperSède pas l’ADR entière)
- Modèle trust : [0305](0305-account-identity-trust-domain.md) ; config signer : [0306](0306-hive-iam-configuration-signature.md)
- Accepted / Partial : [0307](0307-workload-identity-port.md), [0308](0308-mesh-authn-jwt.md) (mode passerelle §7) ; Accepted / Unimplemented : [0309](0309-remote-signer.md)
- IdP : [0400](0400-iamrusty-identite-hexagonale.md)
- Handbook : `docs/platform/authn-jwt.md`
- Non-canon : `IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md`
- Preuves runtime (Partial — pas Implemented : BYOKMS cloud + remote signer 0309 absents) :
  - Encodeur RS256 + SigningProvider : `IAMRusty/infra/src/token/jwt_encoder.rs`, `IAMRusty/infra/src/signing/{pem,transit,static_credential}.rs`
  - JWKS registry : `IAMRusty/infra/src/repository/signing_key_registry.rs` `SeaOrmSigningKeyRegistry::list_jwks_keys` → `TokenUseCaseImpl::get_jwks` → `GET /iam/.well-known/jwks.json`
  - Extracteur RS256 + JWKS + `JwtPrincipal` : `rustycog/rustycog-http/src/jwt_handler.rs`
  - Round-trip sans postgres : `IAMRusty/infra/tests/rs256_jwks_roundtrip.rs`
  - Config `allowed_algorithms=[RS256]` : `IAMRusty/configuration/src/lib.rs` `http_verifier_auth_rs256_only_excludes_hs256` ; HS256 seulement via `allowed_algorithms` dans `*/config/test.toml`
  - Probe test-signer : `IAMRusty/domain/src/port/signing.rs` `OrganizationSignerProbe` ; `IAMRusty/infra/src/signing/probe.rs` ; câblé `IAMRusty/setup/src/app.rs` `SignerRouteContext`
  - Refresh opaque inchangé : `IAMRusty/domain/src/service/refresh_token_service.rs`
