---
title: "ADR 0304–0309 — JWT d'accès, trust et signature"
category: decisions
tags: [architecture, iam, jwt, security, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md
  - docs/adr/0305-account-identity-trust-domain.md
  - docs/adr/0306-hive-iam-configuration-signature.md
  - docs/adr/0307-workload-identity-port.md
  - docs/adr/0308-mesh-authn-jwt.md
  - docs/adr/0309-remote-signer.md
summary: >-
  Cible access JWT : RS256, kid opaque, JWKS, issuer par trust domain.
  0304 Partial ; 0305 et 0306 Implemented ; 0307 Partial ; 0308 et 0309 non livrés.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-27T09:20:00Z
updated: 2026-09-27T09:20:00Z
---

# ADR 0304–0309 — JWT d'accès, trust et signature

Canon : `docs/adr/0304`–`0309`. Hub : [[projects/aiforall/decisions/index]]. Photo antérieure (HS256, `iss=iamrusty`) : [[projects/aiforall/decisions/0300-events-authz]]. How-to : [[projects/aiforall/concepts/jwt-issuer-vs-consumer]].

`Accepted` ratifie une cible. `Réalité` décrit le dépôt au 27 septembre 2026 (HEAD `2473baa`).

## 0304 — RS256 + JWKS (Accepted / Partial)

Acceptation humaine 2026-09-26. SuperSède **uniquement** la cible 0302 « `iss=iamrusty` unique ». AuthZ OpenFGA, Bearer et `aud=aiforall` restent ceux de 0302.

- Access token utilisateur = RS256 + `kid` opaque + JWKS. HS256 interdit après la fenêtre de migration. Pas de try-all-algorithms.
- Quatre rôles : émetteur logique IAMRusty, propriétaire de la clé (plateforme ou org), `SigningProvider`, JWKS canonique `GET /iam/.well-known/jwks.json`.
- KMS seulement sur login / refresh. OpenBao Transit est un `SigningProvider` autorisé (clé non exportée). PEM IAM = mode dev.
- Principal = `(iss, sub)`. Claim `org` = contexte de trust, pas une permission. `kid` ne contient ni org id, ni ARN, ni chemin OpenBao.
- Issuer cible : platform `https://{host}/iam` ; org `https://{host}/iam/orgs/{slug}`. Signature valide + mauvais issuer = rejet.

Réalité **Partial** : mint RS256, JWKS, extracteur rustycog RS256, rotate N+1, probe Transit. Pas d’adapters cloud BYOKMS. Remote signer absent.

## 0305 — Account ≠ Identity (Accepted / Implemented)

`HumanAccount` n’est pas une `Identity`. Principal = `(iss, sub)`. Trust `platform` distinct du trust organization-managed. Table `identities` + `ensure_platform_identity` sur login/refresh.

## 0306 — Config signature Hive→IAM (Accepted / Implemented)

Commande synchrone HTTP Hive→IAM. Pas de secret dans les domain events. Pas Telegraph, pas de bus de commandes.

Commandes : `ConfigureOrganizationSigner`, `TestOrganizationSigner`, `RotateOrganizationSigner`, `DisableOrganizationSigner`. Hive ne garde que `signing_profile_id` et `signing_status`. IAM garde `SigningKey` et le secret (OpenBao).

HEAD `2473baa` : commandes Hive `organization_signer` + client `Hive/infra/src/iam/organization_signer_client.rs`. Revue S6–S8 (27 sept.) : correctness PASS ; le HIGH « PEM racine partagée » du 26 sept. est fermé (préfixe `{org_id}/`). Résidu de durcissement FS noté, non bloquant.

## 0307 — Port WorkloadIdentity (Proposed / Partial)

Port conceptuel. SPIFFE est évalué, pas une dépendance obligatoire, et pas une dépendance de 0304. Réalité **Partial** : port `WorkloadIdentity` + `StaticCredential`. Distinct du certificat workload Lazaret : [[projects/lazaret/concepts/workload-identity]].

## 0308 — Mesh AuthN JWT (Proposed / Unimplemented)

Le gateway valide `alg`, `kid`, signature, `typ`, `iss`, `aud`, `exp`, trust scope, puis produit `(iss, sub)`. Pas livré.

## 0309 — Remote signer (Proposed / Unimplemented)

Contrat minimal `Sign` / `GetPublicKey` derrière HSM ou KMIP. Pas de remote signer JWT dans le dépôt.

## Related

- [[projects/iamrusty/iamrusty]]
- [[projects/hive/hive]]
- [[projects/aiforall/concepts/https-platform-mesh]]
- [[journal/2026-09-27]]
