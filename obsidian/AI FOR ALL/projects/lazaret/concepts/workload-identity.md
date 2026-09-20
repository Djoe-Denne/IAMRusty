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
  - Lazaret/tests/apparatus_p3_t11b_session_mtls.rs
  - Lazaret/tests/apparatus_p3_t13_ca_persist.rs
summary: >-
  Enroll CSR puis session EdDSA. T11b : /session sous mTLS optionnel
  (même CA). T13 : CA persist generate-if-absent, leaf serveur, TTL
  8760 h non figé.
provenance:
  extracted: 0.86
  inferred: 0.12
  ambiguous: 0.02
created: 2026-09-17T10:55:00Z
updated: 2026-09-20T10:35:00Z
---

# Identité workload Lazaret

Canon : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Hub : [[projects/lazaret/lazaret]]. Mesh plateforme (CA **autre**) : [[projects/aiforall/concepts/https-platform-mesh]].

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

## T10 — persist Postgres (`9e85edd`)

L’enrollment n’est plus seulement in-memory : table `apparatus_enrollments` côté Postgres Lazaret. Révocation sur Manifesto `component_removed`. Le hole T3 in-memory est **fermé**. Land : `9e85edd` (le message de commit « cursor review briefing » est trompeur). ^[extracted]

## T11b — `/session` sous mTLS (`d605377`)

Trou mTLS `POST /lazaret/session` **fermé**. HTTPS live via `serve_router` + authentification client **optionnelle** rustycog (pin `0858eab`). Mapping `PeerClientCertificate` → `VerifiedClientCertificate` **sans** écraser les `extensions_mut` T3/T10 : la session mTLS ne doit pas perdre l’identité déjà posée à l’enroll. ^[inferred]

Même **instance** CA pour enroll et `tls_client_ca_path` : le certificat émis à l’enroll est celui que le handshake `/session` vérifie. Preuve : `Lazaret/tests/apparatus_p3_t11b_session_mtls.rs`.

## T13 — persistance CA + leaf serveur (`311e0ab`)

Trou persistance clé CA **fermé**. `generate-if-absent` au boot `Application::new` — pas `openssl` dans l’entrypoint. La leaf serveur est signée par cette CA. Survive un redémarrage avec la **même** CA. Preuve : `Lazaret/tests/apparatus_p3_t13_ca_persist.rs`.

Défaut TTL CA : 24×365 h (8760). Nombre d’**implémentation**, **non** figé par Accept. ^[extracted]

TLS compose Lazaret (distinct du mesh T14b) : `tls_port` 8080, volume `./Lazaret/certs:/app/certs`, HEALTHCHECK `curl -fk https://localhost:8080/lazaret/health`.

## Related

- [[projects/lazaret/concepts/grants-secrets-and-named-proxy]]
- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]] — émetteur IAM vs consommateur
- [[projects/iamrusty/iamrusty]]
- [[journal/2026-09-20]]
