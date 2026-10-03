---
title: >-
  Clôture 0308 — discussion du 2026-10-02
category: references
tags: [mesh, kind, adr, visibility/internal]
sources:
  - docs/adr/0308-mesh-authn-jwt.md
  - docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md
  - docs/0308-mesh-close-implementation-prompt.md
summary: >-
  Discussion du 2 oct. : canaux IT/e2e/isoprod, e2e Kind vert, prompt
  pour passer 0308 à Implemented sans TTL ni mesh défaut.
provenance:
  extracted: 0.84
  inferred: 0.14
  ambiguous: 0.02
created: 2026-10-02T14:55:00Z
updated: 2026-10-02T14:55:00Z
---

# Clôture 0308 — discussion du 2026-10-02

Journal : [[journal/2026-10-02]]. Canon : `docs/adr/0308-mesh-authn-jwt.md`. Prompt : `docs/0308-mesh-close-implementation-prompt.md`.

## Ce que la discussion a tranché

Trois architectes, puis réconciliation. Seul 0308 porte l’amendement de canal. 0304 et 0306 inchangés. 0307 reste Implemented. 0604 garde la photo J3 (`host.docker.internal:5432`) et renvoie la cible isoprod vers 0308. 0605 n’a pas été amendée.

Le mesh opt-in et le Flux commenté ne bloquent plus Implemented. La staleness reste Non décidé, sans chiffre.

## Progrès de code dans le working tree

Healthchecks préfixés, JWKS Compose vers `iam-service`, ports hôte retirés dans l’overlay mesh, routes publiques Envoy sans Check, `user_id` Telegraph, `issuer` Hive, 401 des lectures optionnelles hors SAN `envoy-mesh`. Rien de cela n’est commité. Le gitlink reste `2290d45`.

## Suite

Le prompt de clôture ne couvre que le S2S `token`/`revoke`, le commit du delta rustycog plus le gitlink, le rejeu Kind, puis `Réalité : Implemented`.
