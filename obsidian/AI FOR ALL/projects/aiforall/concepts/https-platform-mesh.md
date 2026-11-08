---
title: >-
  Mesh HTTPS plateforme (T14b)
category: concepts
tags: [architecture, platform, rust, visibility/internal]
sources:
  - services/Hive/tests/https_mesh_optional_mtls.rs
  - services/IAMRusty/tests/https_mesh_optional_mtls.rs
  - services/Telegraph/tests/https_mesh_optional_mtls.rs
  - ops/scripts/generate-platform-mesh-certs.sh
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - ops/deploy/mesh/compose.yaml
  - services/Manifesto/config/development.toml
  - docs/missions/20261108-ops-oauthconnect.md
  - ops/deploy/render-local-full.py
summary: >-
  T14b distingue CA transport et workload. Le profil OAuthConnect ajoute une entrée navigateur HTTPS sans certificat client, en conservant le mTLS upstream ; live non qualifié.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-20T10:35:00Z
updated: 2026-11-08
---

# Mesh HTTPS plateforme (T14b)

Hole mTLS Hive–IAM–Telegraph **fermé** comme HTTPS compose + CA client **optionnelle**. Ce n’est **pas** un rustls `required` de production. Land : `a27ea5b`. Hub : [[projects/aiforall/aiforall]]. Canon P3 : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]].

## Dual-bind rustycog

Chaque slice mesh écoute HTTP clair **et** TLS : `port` 8080 + `tls_port` 8443 côté processus (pin rustycog dual-bind). Les hôtes compose exposent **IAM 8443**, **Telegraph 8444**, **Hive 8445**, **Manifesto 8448** (`8448:8443`). Détail SDK : [[projects/rustycog/rustycog]].

## Deux CA, pas une

La CA mesh `generate-if-absent` sous `./ops/certs/platform-mesh` est **distincte** de la CA Lazaret (identité workload, T13). Script `ops/scripts/generate-platform-mesh-certs.sh` ; service compose `platform-mesh-certs`. Mélanger les deux PKI ferait accepter un certificat d’enrollment comme pair mesh, ou l’inverse. ^[inferred]

Identité workload Lazaret : [[projects/lazaret/concepts/workload-identity]]. Hub frontière : [[projects/lazaret/lazaret]].

## mTLS optionnel (T14b) vs requis (profil mesh)

L’authentification client T14b est branchée seulement si une CA client est fournie. Absent de CA client : HTTPS serveur, pas d’exigence de certificat pair. T14b ferme le trou « pas de TLS compose entre les slices », pas une politique mTLS obligatoire.

Le profil `--profile mesh` est autre chose : `ops/deploy/mesh/compose.yaml` pose `TLS_REQUIRE_CLIENT_CERT=true` sur iam/hive/telegraph/manifesto, et `generate-platform-mesh-certs.sh` émet aussi `manifesto-service`. AuthN JWT derrière Envoy : [[projects/aiforall/concepts/mesh-ext-authz-opt-in]].

## Entrée navigateur HTTPS du nouveau profil local

La photographie T14b/Compose ci-dessus ne décrit pas tous les profils actuels.
Le8novembre, `--oauth-connect` est livré avec une entrée
`https://127.0.0.1:18080`, feuille Envoy avec SAN IP existante et **sans certificat
client navigateur**. Les transport_socket upstream gardent le mTLS du profil
Kind ; enlever l'exigence client à l'entrée ne revient pas à enlever le TLS des
services.

Callbacks publics, cookie/issuer IAM et consommateurs sont alignés. GitHub garde
le contrôle CA/SAN ; la préférence utilisateur pour HTTPS évite d'introduire une
exception callback dans le validateur. Les profils historiques restent HTTP à
l'entrée ; aucune réécriture de leur preuve n'est faite.

Confiance navigateur à la CA, enregistrement GitHub et handshake réel restent
non exécutés. Lire [[projects/oauthconnect/concepts/tls-hmac-and-signatures]] et
[[projects/oauthconnect/concepts/local-ops-and-secret-distribution]].

## Preuves

`services/Hive/tests/https_mesh_optional_mtls.rs`, `services/IAMRusty/tests/https_mesh_optional_mtls.rs`, `services/Telegraph/tests/https_mesh_optional_mtls.rs`. Commandes : [[projects/lazaret/skills/running-apparatus-p3-tests]].

## Related

- [[projects/hive/hive]]
- [[projects/iamrusty/iamrusty]]
- [[projects/telegraph/telegraph]]
- [[projects/manifesto/manifesto]]
- [[projects/aiforall/concepts/mesh-gateway-principal-trust]]
- [[journal/2026-09-30]]
- [[journal/2026-09-20]]
