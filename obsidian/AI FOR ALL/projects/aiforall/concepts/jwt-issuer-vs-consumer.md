---
title: >-
  JWT issuer versus consumer
category: concepts
tags: [iam, jwt, rustycog, security, visibility/internal]
sources:
  - docs/platform/authn-jwt.md
  - docs/guides/jwt-consommateur.md
  - services/IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md
  - rustycog/rustycog-http/src/jwt_handler.rs
  - projects/iamrusty/concepts/jwt-algorithm-enforcement-and-test-relaxation.md
  - cursor-conversation/jwt-jwks-unification-2026-08-29
  - docs/adr/0302-authn-jwt-authz-openfga.md
  - docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md
  - docs/adr/0308-mesh-authn-jwt.md
  - docs/guides/iam-oauthconnect-securite-sources.md
summary: >-
  Cible 0304 : access RS256, kid opaque, JWKS, issuer par trust domain.
  Réalité Partial. HS256 = fenêtre de migration, plus le contrat cible.
provenance:
  extracted: 0.84
  inferred: 0.12
  ambiguous: 0.04
created: 2026-08-31T13:30:00Z
updated: 2026-11-08
---

# JWT issuer versus consumer

How-to (GitHub handbook, not this page): `docs/platform/authn-jwt.md` and `docs/guides/jwt-consommateur.md`.

## Actualisation ciblée du8novembre2026

Le [registre de sources S11–S14](../../../../../docs/guides/iam-oauthconnect-securite-sources.md) suit les corps actuels : IAM délègue la signature à Transit ou au signer distant configuré, sans fallback local après erreur du provider choisi. La déclaration historique «remote signer absent» ci-dessous n'est donc plus vraie pour le code actuel. Cette lecture ne constitue pas un nouvel audit global BYOKMS/mesh ni une modification de statut ADR.

La rotation organisation suit Admin→Hive→IAM→Transit ; **signature distante et Rotate sont des capacités distinctes**. La primitive de rotation inspectée refuse RemoteHttp/cloud. Prépublication Pending, promotion writer Active/Retiring et fence finale d'émission sont vérifiées en sources ; la rétention publique utilise le TTL JWT IAM configuré+60s, pas le cache300s/30s de Connect. Aucun calendrier ou coffre live n'est inféré.

Ne pas confondre les deux rôles : OAuthConnect **consomme** les publics du fournisseur OIDC pour vérifier ses ID tokens ; IAM **émet** les tokens plateforme et publie ses propres publics. Voir [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]] et [[projects/oauthconnect/concepts/tls-hmac-and-signatures]].

## Contrat et photographie historique (2026-09-27)

Cible : [[projects/aiforall/decisions/0304-access-jwt-trust]] (ADR-0304 Accepted / Réalité Partial).

- Access token = RS256 + `kid` opaque. JWKS canonique `GET /iam/.well-known/jwks.json`. Principal = `(iss, sub)`. `aud` reste `aiforall`.
- L’émetteur logique est IAMRusty. L’opération crypto est un `SigningProvider` (OpenBao Transit autorisé ; PEM = dev). Les consommateurs ne connaissent pas le KMS.
- Issuer cible : `https://{host}/iam` (platform) ou `https://{host}/iam/orgs/{slug}` (org). Cela remplace, pour la cible seulement, `iss=iamrusty`.
- Réalité Partial à cette date : mint RS256, JWKS, extracteur rustycog RS256, rotate N+1, probe Transit. Le constat alors fait sur l'absence de remote signer est supersédé par la lecture ciblée du8novembre ci-dessus ; le périmètre cloud n'est pas réaudité ici.
- HS256 et `iss=iamrusty` restent la fenêtre de migration / les tokens historiques, plus le contrat à écrire. La photo du 1er sept. (extracteur HS256-only, refus RS256) est caduque comme cible.

## Limites, avec leur portée

- Le signer distant est présent dans les sources courantes ; les capacités cloud et leur qualification opérationnelle ne sont pas établies par cette page. Aucune capacité Rotate remote/cloud ne se déduit du seul port Sign.
- Le mesh qui valide puis émet `(iss, sub)` est ADR-0308 **Accepted / Partial** : opt-in ext_authz. Sur Compose, mTLS de hop + mode passerelle §7 sont livrés ; le défaut reste JWT in-process. [[projects/aiforall/concepts/mesh-ext-authz-opt-in]] [[projects/aiforall/concepts/mesh-gateway-principal-trust]]

## Related

- [[projects/aiforall/decisions/0300-events-authz]] — ADR 0302 (AuthZ inchangé)
- [[projects/aiforall/decisions/0304-access-jwt-trust]] — cible 0304–0309
- [[projects/aiforall/decisions/0308-mesh-authn-jwt]] — overlay mesh §7
- [[concepts/architecture-coherence-across-services]]
- [[projects/iamrusty/iamrusty]]
- [[skills/using-rustycog-http]]
- [[projects/iamrusty/references/iamrusty-runtime-and-security]]
