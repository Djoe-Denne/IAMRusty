# ADR-0304 : Access JWT = RS256 + kid opaque + JWKS ; SigningProvider ; issuer par trust domain

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Amendement : 2026-10-03 — arbitrage utilisateur fail-closed JWKS à 60 s, cohérent avec [0308](0308-mesh-authn-jwt.md) point 6 ; cible ratifiée, preuve d’implémentation encore à produire.
- Jalon concerné : architecture actuelle / AuthN JWT (complète 0302 ; hors P-Apparatus)
- SuperSède : la décision antérieure **de cette ADR** « OpenBao ne signe pas les JWT utilisateurs » ; **pour la cible seulement**, la décision [0302](0302-authn-jwt-authz-openfga.md) « `iss=iamrusty` unique » (pas l’ADR 0302 entière — AuthZ OpenFGA, Bearer, `aud=aiforall` restent)
- SuperSédée par : —
- Related : [0302](0302-authn-jwt-authz-openfga.md), [0305](0305-account-identity-trust-domain.md), [0306](0306-hive-iam-configuration-signature.md), [0307](0307-workload-identity-port.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md), [0400](0400-iamrusty-identite-hexagonale.md)

`Accepted` ratifie la cible crypto / JWKS / issuer par trust domain. `Réalité : Partial` au 2026-10-04 : sources IAM et SDK de fail-closed60, JWKS vide et mint RS256 commun écrites ; validation intégrée IAM/mesh encore absente. Le SDK publié `ca2e35fcd56279e9e52625d0df9381f240f3390d` est sélectionné dans le worktree ; 22 tests purs SDK PASS rapportés par le parent ne sont pas des tests IAM. Remote signer HTTP (0309) présent / Partial ; adapters cloud BYOKMS et HSM/KMIP non livrés, pas des gates du lab local.

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
17. **Cache JWKS** : pas de fetch par requête ; `kid` connu = validation locale ; `kid` inconnu = refresh coalescé (singleflight), negative cache, last-known-good, refresh périodique. Détail mesh = [0308](0308-mesh-authn-jwt.md). Consommateurs ne parlent pas au KMS. Amendements 2026-10-02 / 2026-10-03, détails dans 0308 point 6 : poll 60 s en staging / prod, 2 s en local / Kind / tests ; **âge maximal de confiance 60 s depuis le dernier snapshot autoritatif validé**, même pendant une panne de refresh. À la borne : fail-closed, sans prolongation par cache hit ni refresh échoué. Règle identique en mesh et in-process. JWKS valide vide = retrait immédiat des clés ; registry initialisé vide ≠ bootstrap jamais initialisé. La révocation de la dernière clé ne republie pas une clé bootstrap et une clé révoquée ne sert plus à émettre. Décision ratifiée ; implémentation et preuve finales encore requises.
18. **`typ` cible = `aiforall-access+jwt`** (rien dans le runtime ne prouve `at+jwt`). Décision, pas implémenté.
19. **Port conceptuel `SigningProvider`** (`sign_digest`, `public_key`, `capabilities`) ; algo domaine RS256 ; mapping vendor dans l’adapter. Séparer `CredentialProvider` / `WorkloadIdentity` ([0307](0307-workload-identity-port.md)). WIF (OIDC / X509) préféré. `StaticCredential` = fallback ; secret dans OpenBao ; jamais Hive DB ; jamais domain event. Ordre : OIDC WIF, X509/mTLS, static.
20. **JWKS unique** acceptable maintenant ; limite ~1000+ à surveiller ; 10000+ ⇒ évolution sharding / JWKS par issuer. Pas de surconception.
21. **SPIFFE** : pas dépendance obligatoire. Renvoi [0307](0307-workload-identity-port.md). Le domaine dépend d’un port `WorkloadIdentity`.
22. **Remote signer** : contrat minimal → [0309](0309-remote-signer.md).
23. **`services/IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md` n’est pas canon.**

## État source et validation courante

Sources courantes : access JWT RS256, `kid` opaque, `typ=aiforall-access+jwt`, `iss={public_base_url}/iam`, `aud=aiforall`. Le codec IAM partage désormais la signature/fence writer entre access et registration ; Active seul émet, contrôle autoritatif final après les await du signer, point L = snapshot du SELECT primaire. Le provider PEM valide la paire privée/publique ; le candidat doit vérifier contre le public enregistré. Registry/JWKS sérialisent status/trust/organisation et un registry initialisé sans clé admissible publie `keys: []`, sans résurrection bootstrap. Ces éléments sont SOURCE, pas une preuve de compilation ou de concurrence backend. À la baseline historique `f060d47` / `ba69c9e`, fallback bootstrap sur registry vide et LKG non borné étaient des écarts ; ils ne décrivent plus les nouvelles sources. Le remote signer HTTP reste Partial ; cloud BYOKMS/HSM/KMIP non livrés. HS256 reste la compat de tests explicite, pas le défaut RS256 runtime.

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

## Réconciliation source — 2026-10-04

- Root HEAD `5348a63`, changements non committés ; checkout/gitlink worktree SDK `ca2e35fcd56279e9e52625d0df9381f240f3390d`, publication confirmée par le parent. Aucun commit/push root ni nouveau run effectué par cette réconciliation.
- Source B : `services/IAMRusty/infra/src/token/{jwt_encoder,registration_token_service}.rs`, `infra/src/signing/pem.rs`, `infra/src/repository/signing_key_registry.rs` (préfixe services/IAMRusty) ; review CORE final PASS **statique**. Codec registration async et bindings shared setup restent à intégrer/valider avec le lot final.
- Source caches : `rustycog/rustycog-http/src/jwks.rs`, `workers/ext-authz/src/jwks_cache.rs` et validateurs de trust ; succès autoritatif vide et confiance monotone limitée à 60 s, hit/erreur sans prolongation. Le parent rapporte 22 tests purs SDK PASS, y compris contrat de publication valide/métadonnées ; cela ne prouve pas le publisher IAM, ses transactions ou Envoy.
- Gates encore ouverts : compilation root complète après intégration, units IAM effectivement exécutées, IT PostgreSQL/publisher/fence/rotation/reset et E2E mesh au hash courant. Pas de promotion Implemented sur source/review. Cloud/HSM futurs ne sont pas des conditions de ces gates locaux.
- Provenance : `.cursor/review-briefings/20261004-package-b-registration-final-wiring.md`, `20261004-correctness-iam-core-final.md`, contrat interface §§12–13 et validation parent SDK22 du 2026-10-04. `f060d47` reste une baseline historique.

## Références

- Complète : [0302](0302-authn-jwt-authz-openfga.md) (AuthN/AuthZ ; ne SuperSède pas l’ADR entière)
- Modèle trust : [0305](0305-account-identity-trust-domain.md) ; config signer : [0306](0306-hive-iam-configuration-signature.md)
- Accepted / Implemented : [0307](0307-workload-identity-port.md), port WIF seulement ; Accepted / Partial : [0308](0308-mesh-authn-jwt.md) (mode passerelle §7), [0309](0309-remote-signer.md) (HTTP, sans HSM/KMIP livré)
- IdP : [0400](0400-iamrusty-identite-hexagonale.md)
- Handbook : `docs/platform/authn-jwt.md`
- Non-canon : `services/IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md`
- Sources et tests présents (pas de nouveau résultat IAM revendiqué ; remote signer HTTP Partial, BYOKMS cloud non livré) :
  - Encodeur RS256 + SigningProvider : `services/IAMRusty/infra/src/token/jwt_encoder.rs`, `services/IAMRusty/infra/src/signing/{pem,transit,static_credential}.rs`
  - JWKS registry : `services/IAMRusty/infra/src/repository/signing_key_registry.rs` `SeaOrmSigningKeyRegistry::list_jwks_keys` → `TokenUseCaseImpl::get_jwks` → `GET /iam/.well-known/jwks.json`
  - Extracteur RS256 + JWKS + `JwtPrincipal` : `rustycog/rustycog-http/src/jwt_handler.rs`
  - Round-trip sans postgres : `services/IAMRusty/infra/tests/rs256_jwks_roundtrip.rs`
  - Config `allowed_algorithms=[RS256]` : `services/IAMRusty/configuration/src/lib.rs` `http_verifier_auth_rs256_only_excludes_hs256` ; HS256 seulement via `allowed_algorithms` dans `*/config/test.toml`
  - Probe test-signer : `services/IAMRusty/domain/src/port/signing.rs` `OrganizationSignerProbe` ; `services/IAMRusty/infra/src/signing/probe.rs` ; câblé `services/IAMRusty/setup/src/app.rs` `SignerRouteContext`
  - Refresh opaque inchangé : `services/IAMRusty/domain/src/service/refresh_token_service.rs`

## Mise à jour 2026-10-04 — migrations aplaties

Il n'existe pas de données en production à préserver. Le schéma IAM est livré en un seul fichier de migration initiale, `services/IAMRusty/migration/src/m20220101_000001_initial_schema.rs`, outbox comprise. Le préflight et le backfill de cutover legacy sont supprimés ; les contraintes d'admission restent inchangées. Les migrations incrémentales seront réintroduites seulement quand un état persisté devra être préservé. Statut et réalité de livraison inchangés ; cette décision ne constitue pas une preuve IT/E2E.
