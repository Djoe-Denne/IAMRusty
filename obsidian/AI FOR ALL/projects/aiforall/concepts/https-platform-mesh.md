---
title: >-
  Mesh HTTPS plateforme (T14b)
category: concepts
tags: [architecture, platform, rust, visibility/internal]
sources:
  - Hive/tests/https_mesh_optional_mtls.rs
  - IAMRusty/tests/https_mesh_optional_mtls.rs
  - Telegraph/tests/https_mesh_optional_mtls.rs
  - scripts/generate-platform-mesh-certs.sh
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
summary: >-
  Mesh HTTPS T14b : dual-bind rustycog 8080+8443, CA platform-mesh ≠ CA
  Lazaret, mTLS client optionnel. Hive 8443, IAM 8444, Telegraph 8445.
provenance:
  extracted: 0.80
  inferred: 0.16
  ambiguous: 0.04
created: 2026-09-20T10:35:00Z
updated: 2026-09-20T10:35:00Z
---

# Mesh HTTPS plateforme (T14b)

Hole mTLS Hive–IAM–Telegraph **fermé** comme HTTPS compose + CA client **optionnelle**. Ce n’est **pas** un rustls `required` de production. Land : `a27ea5b`. Hub : [[projects/aiforall/aiforall]]. Canon P3 : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]].

## Dual-bind rustycog

Chaque slice mesh écoute HTTP clair **et** TLS : `port` 8080 + `tls_port` 8443 côté processus (pin rustycog dual-bind). Les hôtes compose exposent **Hive 8443**, **IAMRusty 8444**, **Telegraph 8445**. Détail SDK : [[projects/rustycog/rustycog]].

## Deux CA, pas une

La CA mesh `generate-if-absent` sous `./certs/platform-mesh` est **distincte** de la CA Lazaret (identité workload, T13). Script `scripts/generate-platform-mesh-certs.sh` ; service compose `platform-mesh-certs`. Mélanger les deux PKI ferait accepter un certificat d’enrollment comme pair mesh, ou l’inverse. ^[inferred]

Identité workload Lazaret : [[projects/lazaret/concepts/workload-identity]]. Hub frontière : [[projects/lazaret/lazaret]].

## mTLS optionnel

L’authentification client est branchée seulement si une CA client est fournie. Absent de CA client : HTTPS serveur, pas d’exigence de certificat pair. T14b ferme le trou « pas de TLS compose entre les trois slices », pas une politique mTLS obligatoire. ^[extracted]

## Preuves

`Hive/tests/https_mesh_optional_mtls.rs`, `IAMRusty/tests/https_mesh_optional_mtls.rs`, `Telegraph/tests/https_mesh_optional_mtls.rs`. Commandes : [[projects/lazaret/skills/running-apparatus-p3-tests]].

## Related

- [[projects/hive/hive]]
- [[projects/iamrusty/iamrusty]]
- [[projects/telegraph/telegraph]]
- [[journal/2026-09-20]]
