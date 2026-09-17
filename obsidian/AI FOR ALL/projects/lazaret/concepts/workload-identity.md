---
title: >-
  Identité workload Lazaret
category: concepts
tags: [architecture, rust, visibility/internal]
sources:
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - docs/services/lazaret.md
  - Lazaret/domain/src/identity.rs
  - Lazaret/application/src/identity.rs
  - Lazaret/http/src/lib.rs
summary: >-
  Enroll CSR anonyme puis session EdDSA iss/aud=lazaret. Un jeton valide
  n’autorise pas : Manifesto live reste obligatoire.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-17T10:55:00Z
updated: 2026-09-17T10:55:00Z
---

# Identité workload Lazaret

Canon : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Hub : [[projects/lazaret/lazaret]].

## Hybride T3

1. **Enroll** : le workload présente un CSR ; la clé privée reste sur le workload. CA logicielle in-process `platform-internal-ca` (produit, pas une PKI commerciale).
2. **Session** : certificat client vérifié → jeton dédié `iss`=`lazaret`, `aud`=`lazaret`, EdDSA. Ce n’est **jamais** un JWT IAM (`iss=iamrusty`, `aud=aiforall`).
3. **Autorisation** : `AUTHORIZATION_FROM_SESSION = LiveManifestoCheckRequired`. Une session cryptographiquement valide ne suffit pas. ^[extracted]

TTL d’implémentation (pas des nombres figés Accepted) : session 15 min, certificat 24 h. ^[inferred]

Identité publique = `project_components.id`. Pas de second UUID public. Style tables : ids internes BIGSERIAL, `component_id` UUID.

## Revue 16 sept. (chat)

Choix figés sur le code alors non commité, puis landés dans `e978cd0` :

- `GET` snapshot de grants = **JWT de service**, pas une check OpenFGA. ^[inferred]
- Enroll **reste anonyme**, mais consult Manifesto + CSR **forcé**. ^[inferred]

Routes HTTP d’identité : `POST /lazaret/enroll`, `POST /lazaret/session`. Le routeur identité n’empile pas le middleware JWT utilisateur rustycog. `[auth.jwt]` existe pour `UserIdExtractor` / `AppState` seulement.

## Related

- [[projects/lazaret/concepts/grants-secrets-and-named-proxy]]
- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]] — émetteur IAM vs consommateur
- [[projects/iamrusty/iamrusty]]
