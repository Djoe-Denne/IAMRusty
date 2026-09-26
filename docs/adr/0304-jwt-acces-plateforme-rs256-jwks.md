# ADR-0304 : Access JWT = RS256 + kid opaque + JWKS ; SigningProvider ; issuer par trust domain

- Statut : Accepted
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Jalon concerné : architecture actuelle / AuthN JWT (complète 0302 ; hors P-Apparatus)
- SuperSède : la décision antérieure **de cette ADR** « OpenBao ne signe pas les JWT utilisateurs » ; **pour la cible seulement**, la décision [0302](0302-authn-jwt-authz-openfga.md) « `iss=iamrusty` unique » (pas l’ADR 0302 entière — AuthZ OpenFGA, Bearer, `aud=aiforall` restent)
- SuperSédée par : —
- Related : [0302](0302-authn-jwt-authz-openfga.md), [0305](0305-account-identity-trust-domain.md), [0306](0306-hive-iam-configuration-signature.md), [0307](0307-workload-identity-port.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md), [0400](0400-iamrusty-identite-hexagonale.md)

`Accepted` ratifie la cible crypto / JWKS / issuer par trust domain. `Réalité : Unimplemented` : le dépôt émet et vérifie encore HS256. Périmètre = JWT, signature, JWKS, rotation, trust crypto — **pas** le modèle Account/Hive ([0305](0305-account-identity-trust-domain.md), [0306](0306-hive-iam-configuration-signature.md)).

## Contexte

[0302](0302-authn-jwt-authz-openfga.md) fige AuthN Bearer + AuthZ OpenFGA, `iss=iamrusty`, `aud=aiforall`, et laissait ouvert RS256 vs HS256. Aujourd’hui IAM signe HS256 ; `UserIdExtractor` (rustycog-http) ne vérifie que HS256 ; le composition root refuse RS256. Un HMAC partagé est la dette de rotation. La version antérieure de **cette** ADR interdisait OpenBao Transit pour signer les JWT utilisateurs — remplacée ici.

## Décision

1. **Access tokens user = RS256 + `kid` opaque + JWKS.** HS256 access **interdit après migration**. Fenêtre dual-verify bornée : RS256 = clé RSA + `kid` requis ; HS256 = ancienne clé HMAC, migration only. Pas de try-all-algorithms ; jamais une pubkey RSA comme secret HMAC. Après cutoff : `allowed_algorithms=[RS256]`.
2. **Quatre rôles.** (a) **Émetteur logique** = IAMRusty (flux normal) : choisit principal, trust domain, claims + header + signing input, choisit `SigningKey`, demande la signature, assemble le JWT. (b) **Propriétaire / admin de la clé privée** = plateforme **ou** organisation. (c) **Opération crypto** = `SigningProvider`. (d) **IAM publie le JWKS canonique** `GET /iam/.well-known/jwks.json`. Les consommateurs ne connaissent pas OpenBao / AWS / GCP / Azure / Scaleway / HSM.
3. **KMS uniquement sur le chemin LOGIN / REFRESH** (IAM → SigningProvider → KMS). Jamais sur le chemin requête API. Failure domain : KMS ACME down ⇒ pas de nouveaux tokens / refresh ACME ; JWT ACME déjà émis restent vérifiables ; autres orgs + plateforme OK.
4. **OpenBao Transit EST un SigningProvider autorisé** (Sign, clé non exportée). Mode PEM chargé par IAM autorisé pour dev / simple — compromis documenté (clé en mémoire, éventuellement Secret k8s). **Remplace** « OpenBao ne signe pas ».
5. **Clé dédiée par org platform-managed** (ex. OpenBao `platform-key`, `org-acme-key`, …). Révocation d’une clé org n’impacte pas les autres. **Clé plateforme commune** = fallback orgs sans clé dédiée + identités platform-managed.
6. **BYOKMS** : l’org peut garder la propriété (AWS KMS, GCP KMS, Azure KV / Managed HSM, Scaleway, OpenBao client, HSM, KMIP, PKCS#11, remote signer). IAM construit le signing input ; le KMS signe ; IAM assemble. **Limitation acceptée** : org propriétaire de son KMS = autorité crypto de **son** trust domain (peut produire ses propres principals de ce domain). Elle ne peut pas faire accepter un principal platform, ni d’une autre org, ni un autre issuer. Le KMS ne prouve pas que le digest vient d’IAM — seulement qu’un principal est autorisé à signer.
7. **`aud` reste `aiforall`.** `aud` ≠ organisation. Audiences user-plane / admin / billing = Non décidé.
8. **Trust domain par clé** : `platform` | `organization(org_id)`. Registry conceptuelle `SigningKey` : `id`, `kid` opaque, `algorithm`, `trust_scope`, `issuer`, `provider_type`, `provider_key_ref`, `credential_ref` optionnel, `public_key`, `status` (`pending` | `active` | `retiring` | `revoked`), timestamps. `kid` = **public opaque** — jamais org id, ARN, project GCP, resource Azure, chemin OpenBao, URL.
9. **Issuer lié au trust domain.** Cible : platform `https://iam.<domain>/platform` ; org `https://iam.<domain>/orgs/{slug}`. Propriété : `kid` → trust domain → issuer. Signature valide + mauvais issuer = **REJECT**. SuperSède **explicitement uniquement** la décision 0302 « `iss=iamrusty` unique » **pour la cible**. Runtime reste `iss=iamrusty` tant que Unimplemented. Tokens actuels `iss=iamrusty` = mode platform historique jusqu’au cutoff.
10. **Principal canonique = `(iss, sub)`**, jamais `sub` seul.
11. **Claim `org` = contexte de trust, pas permission.** Cohérence `key.owner == org == issuer` du domain = AuthN. OpenFGA = AuthZ. Pas d’org dans `aud`.
12. **Jeton platform-managed** : `iss` platform ; pas forcément réémission par org métier. **Jeton organization-managed** : `iss` org, claim `org`, signé seulement par une clé de ce trust domain.
13. **Rotation = deux durées.** (1) Prépublication N+1 dans le JWKS **avant** de signer avec N+1 (caches découvrent N+1). (2) Rétention N pendant la vie restante des access + clock skew, puis retrait. Pas une seule formule.
14. **Révocation urgence** : `status=revoked`, stop signatures, retrait confiance, propagation validators. Delete JWKS ≠ instantané (cache). Propriété : propagation maximale documentée ; mécanisme détaillé = [0308](0308-mesh-authn-jwt.md). Ne pas confondre révocation clé vs session.
15. **Refresh** : opaques, rotatifs, révocables. Révoquer un refresh n’annule pas les access JWT déjà émis. Pas de deny-list `jti` ici.
16. **Headers `jku`, `x5u`, `jwk`** ignorés / refusés comme trust roots. `kid` ne construit jamais URL / path / host.
17. **Cache JWKS** : pas de fetch par requête ; `kid` connu = validation locale ; `kid` inconnu = refresh coalescé (singleflight), negative cache, last-known-good, refresh périodique. Détail mesh = [0308](0308-mesh-authn-jwt.md). Consommateurs ne parlent pas au KMS.
18. **`typ` cible = `aiforall-access+jwt`** (rien dans le runtime ne prouve `at+jwt`). Décision, pas implémenté.
19. **Port conceptuel `SigningProvider`** (`sign_digest`, `public_key`, `capabilities`) ; algo domaine RS256 ; mapping vendor dans l’adapter. Séparer `CredentialProvider` / `WorkloadIdentity` ([0307](0307-workload-identity-port.md)). WIF (OIDC / X509) préféré. `StaticCredential` = fallback ; secret dans OpenBao ; jamais Hive DB ; jamais domain event. Ordre : OIDC WIF, X509/mTLS, static.
20. **JWKS unique** acceptable maintenant ; limite ~1000+ à surveiller ; 10000+ ⇒ évolution sharding / JWKS par issuer. Pas de surconception.
21. **SPIFFE** : pas dépendance obligatoire. Renvoi [0307](0307-workload-identity-port.md). Le domaine dépend d’un port `WorkloadIdentity`.
22. **Remote signer** : contrat minimal → [0309](0309-remote-signer.md).
23. **`IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md` n’est pas canon.**

## État runtime

HS256 partagé ; refus RS256 au boot (`JwtConfig::http_verifier_auth`) ; JWKS publié mais **aucun consommateur** ; `iss=iamrusty` ; `aud=aiforall` ; pas de `SigningProvider` ; OpenBao Transit = Cosign Apparatus seulement (`scripts/openbao-gold-seed.sh` `apparatus-p4-cosign`) — pas de Sign JWT user.

## Migration

Dual-verify bornée (RS256 + `kid` ; HS256 = HMAC migration only) puis cutoff HS256 (`allowed_algorithms=[RS256]`). Tokens `iss=iamrusty` = platform historique jusqu’au cutoff issuer. Ne jamais publier le HMAC dans le JWKS.

## Conséquences

- Tant que `Réalité` ≠ Implemented : HS256 partagé et refus RS256 au composition root restent le runtime ([0302](0302-authn-jwt-authz-openfga.md), handbook).
- Après Implemented : plus de secret HMAC d’accès entre services ; rotation = JWKS + `kid` ; issuer = trust domain.
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
| SPIFFE obligatoire pour signer | Port WorkloadIdentity (0307) ; pas de dépendance SPIFFE |
| Account + Mesh + SPIFFE dans cette ADR | Périmètre crypto JWT seulement |

## Non décidé ici

- Audiences user-plane / admin / billing.
- Deny-list `jti` / `auth_version` / `session_version` (ADR future).
- Mécanisme mesh de propagation révocation clé ([0308](0308-mesh-authn-jwt.md)).
- Contrat remote signer détaillé ([0309](0309-remote-signer.md)).
- UX switch d’identity / membership `(iss, sub)` ([0305](0305-account-identity-trust-domain.md), [0306](0306-hive-iam-configuration-signature.md)).

## Références

- Complète : [0302](0302-authn-jwt-authz-openfga.md) (AuthN/AuthZ ; ne SuperSède pas l’ADR entière)
- Modèle trust : [0305](0305-account-identity-trust-domain.md) ; config signer : [0306](0306-hive-iam-configuration-signature.md)
- Proposed : [0307](0307-workload-identity-port.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md)
- IdP : [0400](0400-iamrusty-identite-hexagonale.md)
- Handbook : `docs/platform/authn-jwt.md`
- Non-canon : `IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md`
- Preuves runtime (Unimplemented pour **cette** cible) :
  - Encodeur : `IAMRusty/infra/src/token/jwt_encoder.rs` (`JwtTokenService` ; `JwtAlgorithm` HS256|RS256)
  - Boot refuse RS256 : `IAMRusty/setup/src/app.rs` `setup_jwt` → `JwtConfig::http_verifier_auth` (`IAMRusty/configuration/src/lib.rs`)
  - Extracteur HS256-only : `rustycog/rustycog-http/src/jwt_handler.rs` (`UserIdExtractor`, `Validation::new(Algorithm::HS256)`)
  - Claims : `IAMRusty/domain/src/entity/token.rs` `TokenClaims` — `sub`, `username`, `iss`, `aud`, `exp`, `iat`, `jti` ; pas `typ`, pas `org`
  - Refresh opaque 64 bytes + SHA-256 + rotate : `jwt_encoder.rs` / `RefreshToken::hash_token` / `refresh_token_service.rs`
  - JWKS : `GET /iam/.well-known/jwks.json` (`IAMRusty/http/src/lib.rs`) — aucun consommateur
  - SigningProvider / Transit JWT IAM : **ABSENT** ; Transit = Cosign Apparatus
